// WP-56 / WP-56b: Local AI analysis. Privacy-first by construction — the only
// network call this module ever makes is to a user-configured LOCAL runtime,
// never to a remote cloud service. Two runtimes are supported, both local:
// Ollama (default, `http://127.0.0.1:11434`) and any OpenAI-compatible server
// such as LocalAI (`http://127.0.0.1:8080`).
//
// No new HTTP-client dependency was added for either: both APIs are single
// request/response JSON calls, so `ollama::generate` / `ollama::list_models`
// are a small hand-rolled HTTP/1.1 client over `std::net::TcpStream`, keeping
// the dependency tree unchanged. Every pure piece (request building, response
// parsing, HTTP framing, error classification) is unit-tested without a
// network; the live socket call itself can't be exercised in CI without a
// running model server, matching the same disclosed limitation already
// established for WP-50's PostgreSQL connector and WP-52's SMTP client.
pub mod ollama;
