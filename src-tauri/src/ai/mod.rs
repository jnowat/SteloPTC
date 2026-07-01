// WP-56: Local AI analysis. Privacy-first by construction — the only
// network call this module ever makes is to a user-configured local Ollama
// endpoint (default `http://127.0.0.1:11434`), never to a remote service.
// No new HTTP-client dependency was added for this: Ollama's API is a single
// request/response JSON call, so `ollama::generate` is a small hand-rolled
// HTTP/1.1 client over `std::net::TcpStream`, keeping the dependency tree
// unchanged. Every pure piece (request building, response parsing, HTTP
// framing) is unit-tested without a network; the live socket call itself
// can't be exercised in CI without a running Ollama server, matching the
// same disclosed limitation already established for WP-50's PostgreSQL
// connector and WP-52's SMTP client.
pub mod ollama;
