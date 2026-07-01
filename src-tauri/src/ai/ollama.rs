// WP-56: minimal Ollama HTTP client. Every function that doesn't need a live
// socket (request building, response parsing, HTTP/1.1 framing) is a pure
// function and unit-tested below.
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// e.g. "http://127.0.0.1:11434" — always local by design; the frontend
    /// never exposes a way to point this at a remote host.
    pub base_url: String,
    pub text_model: String,
    pub vision_model: String,
    pub timeout: Duration,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:11434".to_string(),
            text_model: "llama3.1".to_string(),
            vision_model: "llava".to_string(),
            timeout: Duration::from_secs(120),
        }
    }
}

/// Builds the JSON body for Ollama's `/api/generate` endpoint. `images_b64`
/// (raw base64, no `data:` prefix) is only included when non-empty, matching
/// Ollama's vision-model calling convention.
pub fn build_generate_request(model: &str, prompt: &str, images_b64: &[String]) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
    });
    if !images_b64.is_empty() {
        body["images"] = serde_json::Value::Array(
            images_b64.iter().map(|s| serde_json::Value::String(s.clone())).collect(),
        );
    }
    body
}

/// Parses Ollama's non-streaming `/api/generate` JSON response body,
/// extracting the `response` field. Returns a clear error for a malformed
/// body or an explicit `{"error": "..."}` response (Ollama's shape when the
/// requested model isn't pulled locally).
pub fn parse_generate_response(body: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("Ollama returned invalid JSON: {}", e))?;
    if let Some(err) = value.get("error").and_then(|v| v.as_str()) {
        return Err(format!("Ollama error: {}", err));
    }
    value
        .get("response")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Ollama response missing 'response' field".to_string())
}

/// Splits a raw `host[:port]` (optionally prefixed with `http://`) into
/// `(host, port)`, defaulting to port 11434 (Ollama's default) when absent.
pub fn parse_host_port(base_url: &str) -> Result<(String, u16), String> {
    let stripped = base_url
        .trim_end_matches('/')
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))
        .unwrap_or(base_url);
    match stripped.split_once(':') {
        Some((host, port_str)) => {
            let port: u16 = port_str.parse().map_err(|_| format!("Invalid port in '{}'", base_url))?;
            Ok((host.to_string(), port))
        }
        None => Ok((stripped.to_string(), 11434)),
    }
}

/// Parses a raw HTTP/1.1 response into `(status_code, body)`. Handles both
/// `Content-Length`-delimited and `Transfer-Encoding: chunked` bodies (Go's
/// `net/http`, which Ollama is built on, uses chunked encoding whenever the
/// handler doesn't set an explicit Content-Length up front).
pub fn parse_http_response(raw: &[u8]) -> Result<(u16, String), String> {
    let text = String::from_utf8_lossy(raw);
    let (head, body_start) = text
        .find("\r\n\r\n")
        .map(|idx| (&text[..idx], idx + 4))
        .ok_or_else(|| "Malformed HTTP response: no header/body separator".to_string())?;

    let mut lines = head.lines();
    let status_line = lines.next().ok_or_else(|| "Malformed HTTP response: no status line".to_string())?;
    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("Malformed HTTP status line: '{}'", status_line))?;

    let is_chunked = lines.clone().any(|l| l.to_ascii_lowercase().starts_with("transfer-encoding: chunked"));
    let body_raw = &text[body_start..];

    let body = if is_chunked {
        dechunk(body_raw)?
    } else {
        body_raw.to_string()
    };

    Ok((status_code, body))
}

/// Decodes an HTTP chunked-transfer-encoded body (hex length lines separated
/// by `\r\n`, terminated by a zero-length chunk).
fn dechunk(chunked: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = chunked;
    while let Some(line_end) = rest.find("\r\n") {
        let size_line = &rest[..line_end];
        let size = usize::from_str_radix(size_line.trim(), 16)
            .map_err(|_| format!("Invalid chunk size line: '{}'", size_line))?;
        if size == 0 {
            break;
        }
        let chunk_start = line_end + 2;
        let chunk_end = chunk_start + size;
        if chunk_end > rest.len() {
            return Err("Chunked body truncated".to_string());
        }
        out.push_str(&rest[chunk_start..chunk_end]);
        rest = rest.get(chunk_end + 2..).unwrap_or("");
    }
    Ok(out)
}

/// Sends a single non-streaming `/api/generate` request to the configured
/// local Ollama instance and returns the model's text response. This is the
/// only function in this module that touches the network — everything it
/// delegates to (`build_generate_request`, `parse_http_response`,
/// `parse_generate_response`) is pure and tested independently.
pub fn generate(config: &OllamaConfig, model: &str, prompt: &str, images_b64: &[String]) -> Result<String, String> {
    let (host, port) = parse_host_port(&config.base_url)?;
    let body = build_generate_request(model, prompt, images_b64).to_string();

    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|e| format!("Could not reach local Ollama at {}:{} — is Ollama running? ({})", host, port, e))?;
    stream.set_read_timeout(Some(config.timeout)).ok();
    stream.set_write_timeout(Some(config.timeout)).ok();

    let request = format!(
        "POST /api/generate HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).map_err(|e| format!("Failed to send request to Ollama: {}", e))?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).map_err(|e| format!("Failed to read response from Ollama: {}", e))?;

    let (status, resp_body) = parse_http_response(&raw)?;
    if status != 200 {
        return Err(format!("Ollama returned HTTP {}: {}", status, resp_body));
    }
    parse_generate_response(&resp_body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_generate_request_omits_images_when_empty() {
        let req = build_generate_request("llama3.1", "Summarize this", &[]);
        assert_eq!(req["model"], "llama3.1");
        assert_eq!(req["stream"], false);
        assert!(req.get("images").is_none());
    }

    #[test]
    fn build_generate_request_includes_images_when_present() {
        let req = build_generate_request("llava", "Describe this photo", &["aGVsbG8=".to_string()]);
        assert_eq!(req["images"][0], "aGVsbG8=");
    }

    #[test]
    fn parse_generate_response_extracts_text() {
        let body = r#"{"model":"llama3.1","response":"This culture looks healthy.","done":true}"#;
        assert_eq!(parse_generate_response(body).unwrap(), "This culture looks healthy.");
    }

    #[test]
    fn parse_generate_response_surfaces_ollama_error() {
        let body = r#"{"error":"model 'llava' not found, try pulling it first"}"#;
        let err = parse_generate_response(body).unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn parse_generate_response_rejects_invalid_json() {
        assert!(parse_generate_response("not json").is_err());
    }

    #[test]
    fn parse_host_port_defaults_to_11434() {
        assert_eq!(parse_host_port("http://127.0.0.1").unwrap(), ("127.0.0.1".to_string(), 11434));
    }

    #[test]
    fn parse_host_port_reads_explicit_port() {
        assert_eq!(parse_host_port("http://localhost:11500/").unwrap(), ("localhost".to_string(), 11500));
    }

    #[test]
    fn parse_http_response_handles_content_length_body() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 13\r\n\r\n{\"ok\":true}\n\n";
        let (status, body) = parse_http_response(raw).unwrap();
        assert_eq!(status, 200);
        assert!(body.starts_with("{\"ok\":true}"));
    }

    #[test]
    fn parse_http_response_handles_chunked_body() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n";
        let (status, body) = parse_http_response(raw).unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, "hello world");
    }

    #[test]
    fn parse_http_response_rejects_malformed_input() {
        assert!(parse_http_response(b"garbage no separator").is_err());
    }
}
