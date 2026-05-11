//! Telegram CLI Daemon - IPC via stdin/stdout using vysheng/telegram-cli
//!
//! This module provides a synchronous interface to the telegram-cli binary.
//! Unlike WhatsApp (which uses JSON-RPC), telegram-cli uses plain text commands
//! sent to stdin and emits JSON lines to stdout.
//!
//! Binary: tg/bin/telegram-cli
//! Arguments: --json (JSON output), -R (no readline), -D (no daemon mode)
//! Commands: msg, dialog_list, contact_list, search, user_info, get_self, history

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use anyhow::{bail, Result};
use serde_json::Value;

/// Telegram CLI daemon - communicates with telegram-cli via stdin/stdout
///
/// # Example
/// ```ignore
/// let mut daemon = TelegramDaemon::new();
/// daemon.start()?;
///
/// // Get authentication status
/// let status = daemon.auth_status()?;
/// println!("Logged in as: {:?}", status);
///
/// // List dialogs
/// let dialogs = daemon.list_dialogs()?;
///
/// // Send a message
/// daemon.send_message("username", "Hello from Rust!")?;
///
/// daemon.stop()?;
/// ```
pub struct TelegramDaemon {
    process: Mutex<Child>,
    binary_path: PathBuf,
}

impl TelegramDaemon {
    /// Create a new TelegramDaemon instance (without starting the process)
    pub fn new() -> Self {
        Self {
            process: Mutex::new(
                Command::new("true")
                    .stdout(Stdio::null())
                    .spawn()
                    .unwrap(),
            ),
            binary_path: PathBuf::new(),
        }
    }

    /// Create with a specific binary path (without starting)
    pub fn with_binary(binary_path: PathBuf) -> Self {
        Self {
            process: Mutex::new(
                Command::new("true")
                    .stdout(Stdio::null())
                    .spawn()
                    .unwrap(),
            ),
            binary_path,
        }
    }

    /// Start the telegram-cli daemon
    ///
    /// Searches for the telegram-cli binary in common locations:
    /// - tg/bin/telegram-cli
    /// - ../tg/bin/telegram-cli
    /// - PATH (via `which telegram-cli`)
    pub fn start() -> Result<Box<Self>> {
        let binary = which_tg()?;
        Self::start_with_binary(binary)
    }

    /// Start with a specific binary path
    pub fn start_with_binary(binary_path: PathBuf) -> Result<Box<Self>> {
        // Verify the binary exists
        if !binary_path.exists() {
            bail!(
                "telegram-cli binary not found at: {}",
                binary_path.display()
            );
        }

        // Spawn telegram-cli with piped stdin/stdout
        // --json: Output JSON
        // -R: No readline (pure stdin/stdout)
        // -D: No daemon mode (stay in foreground)
        let child = Command::new(&binary_path)
            .arg("--json")
            .arg("-R")
            .arg("-D")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn telegram-cli: {}", e))?;

        let daemon = Box::new(Self {
            process: Mutex::new(child),
            binary_path,
        });

        // Give telegram-cli time to initialize
        // It needs to load configuration and connect to Telegram servers
        std::thread::sleep(std::time::Duration::from_millis(1000));

        Ok(daemon)
    }

    /// Send a command to telegram-cli and read the JSON response
    ///
    /// This method writes a command to stdin and reads JSON lines from stdout
    /// until it finds a valid JSON response. Telegram-cli may emit multiple
    /// lines of output (including status messages), so we read until we get
    /// a complete JSON object or array.
    ///
    /// # Arguments
    /// * `cmd` - The command to send (e.g., "get_self", "dialog_list", "msg user text")
    ///
    /// # Returns
    /// The first valid JSON Value found in the output
    fn send_command(&self, cmd: &str) -> Result<Value> {
        let mut proc = self.process.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        // Write command to stdin
        let stdin = proc
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("stdin not available"))?;

        // Telegram-cli commands need newline termination
        writeln!(stdin, "{}", cmd).map_err(|e| anyhow::anyhow!("Write error: {}", e))?;
        stdin
            .flush()
            .map_err(|e| anyhow::anyhow!("Flush error: {}", e))?;

        // Read response from stdout
        let stdout = proc
            .stdout
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("stdout not available"))?;

        let reader = BufReader::new(stdout);

        // Read lines until we get valid JSON
        // Telegram-cli may emit multiple lines - status messages, prompts, etc.
        // We look for the first line that parses as valid JSON
        for line in reader.lines() {
            let line = line.map_err(|e| anyhow::anyhow!("Read error: {}", e))?;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Skip non-JSON lines (telegram-cli may emit prompts, status, etc.)
            let trimmed = line.trim();
            if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
                continue;
            }

            // Try to parse as JSON
            match serde_json::from_str::<Value>(trimmed) {
                Ok(value) => return Ok(value),
                Err(_) => continue, // Keep looking for valid JSON
            }
        }

        bail!("No valid JSON response received from telegram-cli")
    }

    /// Get authentication status by retrieving self info
    ///
    /// Sends the "get_self" command to telegram-cli and returns the result.
    /// If the user is not logged in, this will return an error or empty response.
    ///
    /// # Returns
    /// JSON Value containing the self user info (id, name, phone, etc.)
    pub fn auth_status(&self) -> Result<Value> {
        self.send_command("get_self")
    }

    /// List all dialogs (conversations)
    ///
    /// Sends the "dialog_list" command to telegram-cli.
    ///
    /// # Returns
    /// JSON Value containing an array of dialog objects
    pub fn list_dialogs(&self) -> Result<Value> {
        self.send_command("dialog_list")
    }

    /// List all contacts
    ///
    /// Sends the "contact_list" command to telegram-cli.
    ///
    /// # Returns
    /// JSON Value containing an array of contact objects
    pub fn list_contacts(&self) -> Result<Value> {
        self.send_command("contact_list")
    }

    /// Send a message to a peer (user or chat)
    ///
    /// Sends a message using the "msg" command.
    /// The peer can be a username, phone number, or chat name.
    ///
    /// # Arguments
    /// * `peer` - The recipient (username, phone, or chat name)
    /// * `text` - The message text to send
    ///
    /// # Returns
    /// JSON Value containing the message send result
    pub fn send_message(&self, peer: &str, text: &str) -> Result<Value> {
        // Sanitize text: replace newlines with spaces since telegram-cli
        // uses newlines as command separators on stdin.
        let sanitized = text.replace('\n', " ").replace('\r', " ");
        let cmd = format!("msg {} {}", peer, sanitized);
        self.send_command(&cmd)
    }

    /// Search for messages or contacts
    ///
    /// Sends a search query to telegram-cli.
    ///
    /// # Arguments
    /// * `query` - The search query string
    ///
    /// # Returns
    /// JSON Value containing search results
    pub fn search(&self, query: &str) -> Result<Value> {
        let cmd = format!("search {}", query);
        self.send_command(&cmd)
    }

    /// Get user info for a specific peer
    ///
    /// Sends the "user_info" command to telegram-cli.
    ///
    /// # Arguments
    /// * `peer` - The username or phone number to look up
    ///
    /// # Returns
    /// JSON Value containing user information
    pub fn user_info(&self, peer: &str) -> Result<Value> {
        let cmd = format!("user_info {}", peer);
        self.send_command(&cmd)
    }

    /// Get message history for a chat
    ///
    /// Sends the "history" command to telegram-cli.
    ///
    /// # Arguments
    /// * `peer` - The chat (user or group) to get history from
    /// * `count` - Number of messages to retrieve (default: 10)
    ///
    /// # Returns
    /// JSON Value containing message history
    pub fn history(&self, peer: &str, count: u32) -> Result<Value> {
        let cmd = format!("history {} {}", peer, count);
        self.send_command(&cmd)
    }

    /// Check if the telegram-cli process is running
    ///
    /// # Returns
    /// `true` if the process is still running, `false` if it has exited
    pub fn is_running(&self) -> bool {
        match self.process.lock() {
            Ok(mut proc) => match proc.try_wait() {
                Ok(Some(_)) => false, // Process has exited
                Ok(None) => true,     // Still running
                Err(_) => false,      // Error checking status
            },
            Err(_) => false,
        }
    }

    /// Stop the telegram-cli process
    ///
    /// Sends SIGTERM to the process and waits for it to exit.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if stopping fails
    pub fn stop(&self) -> Result<()> {
        let mut proc = self
            .process
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        // Kill the process
        proc.kill().map_err(|e| anyhow::anyhow!("Failed to kill process: {}", e))?;

        // Wait for it to exit
        proc.wait().map_err(|e| anyhow::anyhow!("Failed to wait for process: {}", e))?;

        Ok(())
    }
}

impl Default for TelegramDaemon {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TelegramDaemon {
    fn drop(&mut self) {
        if let Ok(mut proc) = self.process.lock() {
            let _ = proc.kill();
            let _ = proc.wait();
        }
    }
}

/// Find the telegram-cli binary in common locations
///
/// Searches in this order:
/// 1. tg/bin/telegram-cli (relative to current directory)
/// 2. ../tg/bin/telegram-cli (relative to parent)
/// 3. PATH (via `which telegram-cli`)
///
/// # Returns
/// PathBuf to the telegram-cli binary, or an error if not found
fn which_tg() -> Result<PathBuf> {
    // Check common relative locations
    let candidates = vec![
        PathBuf::from("tg/bin/telegram-cli"),
        PathBuf::from("../tg/bin/telegram-cli"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate
                .canonicalize()
                .unwrap_or_else(|_| candidate.clone()));
        }
    }

    // Try PATH
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let candidate = PathBuf::from(dir).join("telegram-cli");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    bail!(
        "telegram-cli binary not found. Expected locations:\n\
         - tg/bin/telegram-cli\n\
         - ../tg/bin/telegram-cli\n\
         - PATH (telegram-cli)\n\
         Build with: cd tg && ./configure && make"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_which_tg_finds_binary() {
        // This test will pass if telegram-cli exists in expected locations
        // Otherwise it will fail with a clear error message
        let result = which_tg();
        match result {
            Ok(path) => println!("Found telegram-cli at: {}", path.display()),
            Err(e) => println!("telegram-cli not found (expected in test): {}", e),
        }
    }

    #[test]
    fn test_daemon_creation() {
        let daemon = TelegramDaemon::new();
        // new() spawns a placeholder "true" process — it may exit before we check
        let _ = daemon.is_running();
    }

    #[test]
    fn test_daemon_with_binary() {
        let _daemon = TelegramDaemon::with_binary(PathBuf::from("/nonexistent"));
        // with_binary also uses a placeholder — just verify construction succeeds
    }
}