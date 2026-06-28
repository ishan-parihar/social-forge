use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcRequest {
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcResponse {
    pub id: Option<u64>,
    pub result: Option<Value>,
    pub error: Option<IpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcError {
    pub code: i32,
    pub message: String,
}

pub struct WhatsAppDaemon {
    process: Mutex<Child>,
    next_id: AtomicU64,
    binary_path: PathBuf,
    store_dir: PathBuf,
}

impl WhatsAppDaemon {
    pub fn new(binary_path: PathBuf, store_dir: PathBuf) -> Self {
        Self {
            process: Mutex::new(
                Command::new("true").stdout(Stdio::null()).spawn().unwrap(),
            ),
            next_id: AtomicU64::new(1),
            binary_path,
            store_dir,
        }
    }

    pub fn start(store_dir: PathBuf) -> Result<Arc<Self>, String> {
        let binary = which_wacli().map_err(|e| format!("wacli not found: {e}"))?;
        Self::start_with_binary(binary, store_dir)
    }

    pub fn start_with_binary(
        binary_path: PathBuf,
        store_dir: PathBuf,
    ) -> Result<Arc<Self>, String> {
        let child = Command::new(&binary_path)
            .arg("server")
            .arg("--store")
            .arg(&store_dir)
            .arg("--lock-wait")
            .arg("10s")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn wacli: {e}"))?;

        let daemon = Arc::new(Self {
            process: Mutex::new(child),
            next_id: AtomicU64::new(1),
            binary_path,
            store_dir,
        });

        // Give the server time to initialize
        std::thread::sleep(Duration::from_millis(500));

        Ok(daemon)
    }

    pub fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = IpcRequest {
            id,
            method: method.to_string(),
            params,
        };

        let mut proc = self.process.lock().map_err(|e| format!("Lock error: {e}"))?;

        // Write request to stdin
        let stdin = proc
            .stdin
            .as_mut()
            .ok_or_else(|| "stdin not available".to_string())?;
        let req_line =
            serde_json::to_string(&req).map_err(|e| format!("Serialize error: {e}"))?;
        writeln!(stdin, "{}", req_line).map_err(|e| format!("Write error: {e}"))?;
        stdin.flush().map_err(|e| format!("Flush error: {e}"))?;

        // Read response from stdout
        let stdout = proc
            .stdout
            .as_mut()
            .ok_or_else(|| "stdout not available".to_string())?;
        let mut line = String::new();
        let mut reader = BufReader::new(stdout);
        reader
            .read_line(&mut line)
            .map_err(|e| format!("Read error: {e}"))?;

        let resp: IpcResponse =
            serde_json::from_str(&line).map_err(|e| format!("Parse error: {e}"))?;

        if let Some(err) = resp.error {
            return Err(format!("wacli error ({}): {}", err.code, err.message));
        }

        Ok(resp.result.unwrap_or(Value::Null))
    }

    /// Check if WhatsApp is authenticated
    pub fn auth_status(&self) -> Result<serde_json::Value, String> {
        self.send_request("auth_status", None)
    }

    /// Send text message
    pub fn send_text(&self, to: &str, text: &str) -> Result<Value, String> {
        let params = serde_json::json!({
            "to": to,
            "text": text,
        });
        self.send_request("send_text", Some(params))
    }
}

impl Drop for WhatsAppDaemon {
    fn drop(&mut self) {
        if let Ok(mut proc) = self.process.lock() {
            let _ = proc.kill();
            let _ = proc.wait();
        }
    }
}

fn which_wacli() -> Result<PathBuf, String> {
    // Check common locations
    let candidates = vec![
        PathBuf::from("./wacli/dist/wacli"),
        PathBuf::from("../wacli/dist/wacli"),
    ];

    for c in &candidates {
        if c.exists() {
            return Ok(c.canonicalize().unwrap_or_else(|_| c.clone()));
        }
    }

    // Try PATH
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let candidate = PathBuf::from(dir).join("wacli");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    Err("wacli binary not found. Build it with: cd wacli && ./scripts/build-wacli.sh".into())
}
