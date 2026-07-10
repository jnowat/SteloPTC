// WP-56 / WP-56b: minimal local-LLM HTTP client. Every function that doesn't
// need a live socket (request building, response parsing, HTTP/1.1 framing,
// error classification) is a pure function and unit-tested below.
//
// Two providers are supported, both fully local by design:
//   * "ollama"  — Ollama's native API (`/api/generate`, `/api/tags`), the
//                 default. This is the same runtime Gruper drives.
//   * "localai" / "openai" — any OpenAI-compatible server (LocalAI, or Ollama's
//                 own `/v1` compatibility shim), using `/v1/chat/completions`
//                 and `/v1/models`.
// No HTTP-client dependency is added for either: both APIs are single
// request/response JSON calls over `std::net::TcpStream`.
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Provider identifiers accepted in the `ai_provider` setting. Matching is
/// case-insensitive and tolerant of the common aliases so a user typing
/// "OpenAI" or "openai-compatible" still lands on the compatibility path.
pub fn is_openai_compatible(provider: &str) -> bool {
    matches!(
        provider.trim().to_ascii_lowercase().as_str(),
        "localai" | "openai" | "openai-compatible" | "open-webui"
    )
}

#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// Which local runtime to talk to: `"ollama"` (default) or `"localai"`.
    pub provider: String,
    /// e.g. "http://127.0.0.1:11434" (Ollama) or "http://127.0.0.1:8080"
    /// (LocalAI) — always a loopback/LAN host by design; the frontend never
    /// exposes a way to point this at a public cloud service.
    pub base_url: String,
    pub text_model: String,
    pub vision_model: String,
    pub timeout: Duration,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            base_url: "http://127.0.0.1:11434".to_string(),
            text_model: "llama3.1".to_string(),
            vision_model: "llava".to_string(),
            timeout: Duration::from_secs(120),
        }
    }
}

impl OllamaConfig {
    pub fn uses_openai_api(&self) -> bool {
        is_openai_compatible(&self.provider)
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

/// Builds the JSON body for an OpenAI-compatible `/v1/chat/completions` call
/// (LocalAI, or Ollama's `/v1` shim). Vision images are attached to the user
/// message as `image_url` parts with `data:` URIs, the OpenAI convention that
/// LocalAI's vision backends understand.
pub fn build_openai_chat_request(model: &str, prompt: &str, images_b64: &[String]) -> serde_json::Value {
    if images_b64.is_empty() {
        return serde_json::json!({
            "model": model,
            "stream": false,
            "messages": [{ "role": "user", "content": prompt }],
        });
    }
    let mut parts: Vec<serde_json::Value> = vec![serde_json::json!({ "type": "text", "text": prompt })];
    for img in images_b64 {
        parts.push(serde_json::json!({
            "type": "image_url",
            "image_url": { "url": format!("data:image/jpeg;base64,{}", img) },
        }));
    }
    serde_json::json!({
        "model": model,
        "stream": false,
        "messages": [{ "role": "user", "content": parts }],
    })
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

/// Parses an OpenAI-compatible `/v1/chat/completions` response body,
/// extracting `choices[0].message.content`. Surfaces an explicit
/// `{"error": {...}}` body (LocalAI's shape for an unknown model) as an error.
pub fn parse_openai_chat_response(body: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("Server returned invalid JSON: {}", e))?;
    if let Some(err) = value.get("error") {
        let msg = err
            .get("message")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| err.to_string());
        return Err(format!("AI server error: {}", msg));
    }
    value
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Response missing 'choices[0].message.content'".to_string())
}

/// Parses Ollama's `/api/tags` response into the list of locally-installed
/// model names (e.g. `["llama3.1:8b", "llava:latest"]`).
pub fn parse_tags_response(body: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("Ollama returned invalid JSON: {}", e))?;
    let models = value
        .get("models")
        .and_then(|m| m.as_array())
        .ok_or_else(|| "Ollama /api/tags response missing 'models' array".to_string())?;
    Ok(models
        .iter()
        .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
        .collect())
}

/// Parses an OpenAI-compatible `/v1/models` response into the list of model
/// ids (LocalAI, or Ollama's `/v1` shim).
pub fn parse_openai_models_response(body: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("Server returned invalid JSON: {}", e))?;
    let data = value
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "/v1/models response missing 'data' array".to_string())?;
    Ok(data
        .iter()
        .filter_map(|m| m.get("id").and_then(|n| n.as_str()).map(|s| s.to_string()))
        .collect())
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

/// Turns a low-level socket failure into a friendly, actionable message. This
/// mirrors Gruper's `ollama_unreachable` classification: a dead endpoint must
/// be clearly distinguishable from a slow model, so the UI can say "start
/// Ollama" instead of a generic "request failed".
fn unreachable_error(provider: &str, host: &str, port: u16, err: &std::io::Error) -> String {
    let name = if is_openai_compatible(provider) { "AI server (LocalAI)" } else { "Ollama" };
    format!(
        "Could not reach the local {} at {}:{} — is it running? ({})",
        name, host, port, err
    )
}

/// Turns an HTTP error status into a friendly, actionable message. A 404 (or a
/// body mentioning "not found") almost always means the requested model isn't
/// installed, so we surface the exact `ollama pull` command to fix it — the
/// same guidance Gruper gives.
fn classify_status_error(provider: &str, status: u16, body: &str, model: &str) -> String {
    let lowered = body.to_ascii_lowercase();
    if status == 404 || lowered.contains("not found") || lowered.contains("no such model") {
        if is_openai_compatible(provider) {
            return format!(
                "The model \"{}\" isn't available on the local AI server. Install it in LocalAI, or change this model in Settings → AI Assistant.",
                model
            );
        }
        return format!(
            "The model \"{}\" isn't installed in Ollama. Run `ollama pull {}`, or change this model in Settings → AI Assistant.",
            model, model
        );
    }
    let name = if is_openai_compatible(provider) { "The AI server" } else { "Ollama" };
    format!("{} returned HTTP {}: {}", name, status, body)
}

/// Performs one blocking HTTP/1.1 round-trip against the configured local
/// endpoint. `body` is `None` for GET requests and `Some(json)` for POSTs.
/// This is the only function in this module that touches the network —
/// everything it delegates to (request building, response parsing) is pure and
/// tested independently.
fn http_roundtrip(
    config: &OllamaConfig,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<(u16, String), String> {
    let (host, port) = parse_host_port(&config.base_url)?;

    let mut stream = TcpStream::connect((host.as_str(), port))
        .map_err(|e| unreachable_error(&config.provider, &host, port, &e))?;
    stream.set_read_timeout(Some(config.timeout)).ok();
    stream.set_write_timeout(Some(config.timeout)).ok();

    let request = match body {
        Some(json) => format!(
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
            json.len()
        ),
        None => format!(
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
        ),
    };
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("Failed to send request to the local AI runtime: {}", e))?;

    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|e| format!("Failed to read response from the local AI runtime: {}", e))?;

    parse_http_response(&raw)
}

/// Sends a single non-streaming completion request to the configured local
/// runtime and returns the model's text response. Routes to Ollama's
/// `/api/generate` or an OpenAI-compatible `/v1/chat/completions` depending on
/// `config.provider`.
pub fn generate(config: &OllamaConfig, model: &str, prompt: &str, images_b64: &[String]) -> Result<String, String> {
    let (path, body) = if config.uses_openai_api() {
        ("/v1/chat/completions", build_openai_chat_request(model, prompt, images_b64).to_string())
    } else {
        ("/api/generate", build_generate_request(model, prompt, images_b64).to_string())
    };

    let (status, resp_body) = http_roundtrip(config, "POST", path, Some(&body))?;
    if status != 200 {
        return Err(classify_status_error(&config.provider, status, &resp_body, model));
    }
    if config.uses_openai_api() {
        parse_openai_chat_response(&resp_body)
    } else {
        parse_generate_response(&resp_body)
    }
}

/// Lists the models installed on the local runtime. Routes to Ollama's
/// `/api/tags` or an OpenAI-compatible `/v1/models` depending on
/// `config.provider`. Used by the Settings → AI Assistant status check so a
/// user can confirm their runtime is up and see exactly which models are
/// available before running a suggestion.
pub fn list_models(config: &OllamaConfig) -> Result<Vec<String>, String> {
    let path = if config.uses_openai_api() { "/v1/models" } else { "/api/tags" };
    let (status, body) = http_roundtrip(config, "GET", path, None)?;
    if status != 200 {
        let name = if config.uses_openai_api() { "The AI server" } else { "Ollama" };
        return Err(format!("{} returned HTTP {} when listing models: {}", name, status, body));
    }
    if config.uses_openai_api() {
        parse_openai_models_response(&body)
    } else {
        parse_tags_response(&body)
    }
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

    // ── WP-56b: provider routing, OpenAI-compatible (LocalAI) support ────────

    #[test]
    fn is_openai_compatible_recognizes_localai_aliases() {
        assert!(is_openai_compatible("localai"));
        assert!(is_openai_compatible("LocalAI"));
        assert!(is_openai_compatible("openai"));
        assert!(is_openai_compatible("openai-compatible"));
        assert!(!is_openai_compatible("ollama"));
        assert!(!is_openai_compatible(""));
    }

    #[test]
    fn default_config_uses_ollama() {
        let cfg = OllamaConfig::default();
        assert_eq!(cfg.provider, "ollama");
        assert!(!cfg.uses_openai_api());
    }

    #[test]
    fn build_openai_chat_request_text_only() {
        let req = build_openai_chat_request("llama3.1", "Summarize this", &[]);
        assert_eq!(req["model"], "llama3.1");
        assert_eq!(req["stream"], false);
        assert_eq!(req["messages"][0]["role"], "user");
        assert_eq!(req["messages"][0]["content"], "Summarize this");
    }

    #[test]
    fn build_openai_chat_request_attaches_image_as_data_uri() {
        let req = build_openai_chat_request("llava", "Describe", &["aGVsbG8=".to_string()]);
        let parts = &req["messages"][0]["content"];
        assert_eq!(parts[0]["type"], "text");
        assert_eq!(parts[1]["type"], "image_url");
        assert_eq!(parts[1]["image_url"]["url"], "data:image/jpeg;base64,aGVsbG8=");
    }

    #[test]
    fn parse_openai_chat_response_extracts_content() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"Healthy culture."}}]}"#;
        assert_eq!(parse_openai_chat_response(body).unwrap(), "Healthy culture.");
    }

    #[test]
    fn parse_openai_chat_response_surfaces_error_object() {
        let body = r#"{"error":{"message":"model not found","type":"invalid_request_error"}}"#;
        let err = parse_openai_chat_response(body).unwrap_err();
        assert!(err.contains("model not found"));
    }

    #[test]
    fn parse_tags_response_lists_installed_models() {
        let body = r#"{"models":[{"name":"llama3.1:8b"},{"name":"llava:latest"}]}"#;
        assert_eq!(parse_tags_response(body).unwrap(), vec!["llama3.1:8b", "llava:latest"]);
    }

    #[test]
    fn parse_tags_response_empty_when_no_models() {
        let body = r#"{"models":[]}"#;
        assert_eq!(parse_tags_response(body).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn parse_tags_response_rejects_missing_array() {
        assert!(parse_tags_response(r#"{"ok":true}"#).is_err());
    }

    #[test]
    fn parse_openai_models_response_lists_ids() {
        let body = r#"{"object":"list","data":[{"id":"gpt-4","object":"model"},{"id":"llava","object":"model"}]}"#;
        assert_eq!(parse_openai_models_response(body).unwrap(), vec!["gpt-4", "llava"]);
    }

    #[test]
    fn classify_status_error_404_gives_pull_hint_for_ollama() {
        let msg = classify_status_error("ollama", 404, "model not found", "llama3.1");
        assert!(msg.contains("ollama pull llama3.1"));
    }

    #[test]
    fn classify_status_error_404_gives_localai_hint() {
        let msg = classify_status_error("localai", 404, "no such model", "llava");
        assert!(msg.contains("LocalAI"));
        assert!(!msg.contains("ollama pull"));
    }
}
