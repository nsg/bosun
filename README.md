# Bosun

An API gateway that sits between clients and a [Frigate](https://frigate.video)
NVR instance. Bosun authenticates callers by API key and applies a
**default-deny allowlist** over which HTTP method + endpoint combinations each
key may reach, then reverse-proxies permitted requests to Frigate.

## How it works

```
client ──X-API-Key──▶ Bosun ──(allow?)──▶ Frigate
```

1. **Authenticate** — every request (except `/healthz`) must carry a known key
   in the `X-API-Key` header, or it is rejected with `401`.
2. **Authorize (default-deny)** — the request is permitted only if one of the
   key's rules matches **both** the HTTP method and the request path. Otherwise
   `403`.
3. **Proxy** — permitted requests are streamed to Frigate and the response is
   streamed back.

## Configuration

Bosun reads a TOML file (default `bosun.toml`, or pass a path as the first
argument). See [`bosun.example.toml`](bosun.example.toml).

```toml
listen = "0.0.0.0:8080"

[frigate]
url = "http://localhost:5000"

[[api_keys]]
name = "viewer"
key = "change-me"

  [[api_keys.rules]]
  methods = ["GET", "HEAD"]
  paths = ["/api/events", "/api/*/latest.*"]
```

### Rules

Each `[[api_keys.rules]]` block binds a set of HTTP methods to a set of path
patterns. A request is allowed when a single rule matches **both**:

- **`methods`** — HTTP verbs granted, case-insensitive. `"*"` matches any verb.
- **`paths`** — path glob patterns:
  - `*` matches any run of characters **within one segment** (e.g.
    `/api/*/latest.*` matches `/api/front_door/latest.jpg`).
  - `**` matches **any number of segments** (e.g. `/api/**`).

Multiple rules on a key are unioned — the request is allowed if any of them
matches. A key with no rules can access nothing.

## Running

```bash
cargo run -- bosun.toml
```

Make an authenticated request (allowed by the `viewer` example rule):

```bash
curl -H "X-API-Key: change-me" http://localhost:8080/api/events
```

Health check (no auth required):

```bash
curl http://localhost:8080/healthz   # -> ok
```

Logging verbosity is controlled with `RUST_LOG`, e.g.
`RUST_LOG=bosun=debug cargo run -- bosun.toml`.

## Development

```bash
cargo test      # unit + integration tests
cargo clippy
cargo fmt
```

Tests cover Bosun's own behavior — rule matching, config parsing, and the
access-control + proxy flow against a mock upstream — not third-party crates.
