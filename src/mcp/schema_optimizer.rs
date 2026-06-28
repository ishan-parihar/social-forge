// ─── MCP Token Optimization: Schema Optimizer ──────────────────
// Transport-level post-processor that strips JSON Schema bloat from
// MCP tools/list responses. Operates on the newline-delimited JSON-RPC
// stream between rmcp and the LLM client.
//
// Stripped fields: $schema, title, format, minimum, maximum,
// minLength, maxLength, exclusiveMinimum, exclusiveMaximum, multipleOf,
// minItems, maxItems, uniqueItems, pattern
//
// Estimated savings: ~8,500 tokens per tools/list response (~142KB → ~108KB)

use std::io;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, BufReader, ReadBuf};
use tokio::io::{stdin, stdout};
use serde_json::Value;

// ══════════════════════════════════════════════════════════════
// SCHEMA BLOAT STRIPPER
// ══════════════════════════════════════════════════════════════

/// Recursively strip JSON Schema metadata that wastes tokens without
/// helping the LLM understand the schema.
fn strip_schema_bloat(schema: &mut Value) {
    match schema {
        Value::Object(map) => {
            // Remove fields the LLM never uses for tool selection/calling
            map.remove("$schema");
            map.remove("title");
            map.remove("$id");

            // Numeric constraints — LLMs infer these from "type": "integer"
            map.remove("format");
            map.remove("minimum");
            map.remove("maximum");
            map.remove("exclusiveMinimum");
            map.remove("exclusiveMaximum");
            map.remove("multipleOf");

            // Keep minLength, maxLength, pattern — genuinely useful for formatting

            // Array constraints
            map.remove("minItems");
            map.remove("maxItems");
            map.remove("uniqueItems");

            // Recurse into all values (properties, items, $defs, etc.)
            for v in map.values_mut() {
                strip_schema_bloat(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                strip_schema_bloat(v);
            }
        }
        _ => {}
    }
}

/// Strip schema bloat from a tools/list JSON-RPC response.
/// Only modifies responses that contain a "tools" array in "result".
fn strip_tools_list_bloat(msg: &mut Value) {
    if let Some(result) = msg.get_mut("result") {
        if let Some(tools) = result.get_mut("tools") {
            if let Value::Array(tools) = tools {
                for tool in tools {
                    // Strip inputSchema
                    if let Some(schema) = tool.get_mut("inputSchema") {
                        strip_schema_bloat(schema);
                    }
                    // Strip outputSchema (if present)
                    if let Some(schema) = tool.get_mut("outputSchema") {
                        strip_schema_bloat(schema);
                    }
                }
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════
// TRANSPORT WRAPPER
// ══════════════════════════════════════════════════════════════

/// A writer that intercepts outgoing bytes from the MCP transport,
/// buffers complete newline-delimited JSON-RPC messages, strips schema
/// bloat from tools/list responses, and forwards cleaned bytes.
struct SchemaStrippingWriter<W: AsyncWrite + Unpin> {
    inner: W,
    input_buf: Vec<u8>,
    output_buf: Vec<u8>,
}

impl<W: AsyncWrite + Unpin> SchemaStrippingWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            input_buf: Vec::with_capacity(8192),
            output_buf: Vec::with_capacity(8192),
        }
    }

    /// Extract complete newline-terminated messages from input buffer,
    /// strip schema bloat, and append cleaned bytes to output buffer.
    fn process_input(&mut self) {
        while let Some(pos) = self.input_buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.input_buf.drain(..=pos).collect();
            let trimmed = line.trim_ascii_end();

            if trimmed.is_empty() {
                self.output_buf.extend_from_slice(b"\n");
                continue;
            }

            // Try to parse as JSON-RPC and strip schema bloat
            match serde_json::from_slice::<Value>(trimmed) {
                Ok(mut msg) => {
                    strip_tools_list_bloat(&mut msg);
                    match serde_json::to_vec(&msg) {
                        Ok(cleaned) => {
                            self.output_buf.extend_from_slice(&cleaned);
                            self.output_buf.push(b'\n');
                        }
                        Err(_) => self.output_buf.extend_from_slice(&line),
                    }
                }
                Err(_) => self.output_buf.extend_from_slice(&line),
            }
        }
    }

    /// Try to flush output buffer to inner writer.
    /// Returns the number of bytes still buffered.
    fn try_flush_output(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        while !self.output_buf.is_empty() {
            match Pin::new(&mut self.inner).poll_write(cx, &self.output_buf) {
                Poll::Ready(Ok(0)) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "writer returned zero",
                    )));
                }
                Poll::Ready(Ok(n)) => {
                    self.output_buf.drain(..n);
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }
        Poll::Ready(Ok(()))
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for SchemaStrippingWriter<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();

        // First, flush any pending output from a previous call
        if !this.output_buf.is_empty() {
            ready!(this.try_flush_output(cx))?;
        }

        // Process new input bytes
        this.input_buf.extend_from_slice(buf);
        this.process_input();

        // Try to write the cleaned output
        if !this.output_buf.is_empty() {
            match Pin::new(&mut this.inner).poll_write(cx, &this.output_buf) {
                Poll::Ready(Ok(n)) => {
                    this.output_buf.drain(..n);
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => {
                    // Output will be flushed on next poll_write/poll_flush
                }
            }
        }

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.try_flush_output(cx))?;
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        // Flush any remaining stripped bytes before shutdown
        let _ = this.try_flush_output(cx);
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

// ══════════════════════════════════════════════════════════════
// COMBINED TRANSPORT
// ══════════════════════════════════════════════════════════════

/// Combined async transport: reads from stdin (passthrough), writes
/// through schema-stripping wrapper to stdout.
pub struct LeanStdioTransport {
    reader: BufReader<tokio::io::Stdin>,
    writer: SchemaStrippingWriter<tokio::io::Stdout>,
}

impl LeanStdioTransport {
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(stdin()),
            writer: SchemaStrippingWriter::new(stdout()),
        }
    }
}

impl AsyncRead for LeanStdioTransport {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().reader).poll_read(cx, buf)
    }
}

impl AsyncWrite for LeanStdioTransport {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().writer).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().writer).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().writer).poll_shutdown(cx)
    }
}

// ══════════════════════════════════════════════════════════════
// PUBLIC API
// ══════════════════════════════════════════════════════════════

/// Create a schema-optimized stdio transport for MCP.
/// Use this instead of `rmcp::transport::stdio()` to get leaner
/// tools/list responses.
pub fn lean_stdio() -> LeanStdioTransport {
    LeanStdioTransport::new()
}
