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

Bosun is designed to run as a **[Home Assistant add-on](#home-assistant-add-on)**,
where it is configured entirely through Home Assistant's native UI (see below).

For standalone use, Bosun reads a JSON file (default `bosun.json`, or pass a path
as the first argument). See [`bosun.example.json`](bosun.example.json).

```json
{
  "listen": "0.0.0.0:8080",
  "connect_timeout": 10,
  "frigate": { "url": "http://localhost:5000" },
  "api_keys": [
    {
      "name": "viewer",
      "key": "change-me",
      "rules": [
        { "methods": ["GET", "HEAD"], "paths": ["/api/events", "/api/*/latest.*"] }
      ]
    }
  ]
}
```

- **`connect_timeout`** — seconds to wait when opening a connection to Frigate
  (connection setup only; never truncates streamed responses). Defaults to `10`.

### Rules

Each rule binds a set of HTTP methods to a set of path patterns. A request is
allowed when a single rule matches **both**:

- **`methods`** — HTTP verbs granted, case-insensitive. `"*"` matches any verb.
- **`paths`** — path glob patterns:
  - `*` matches any run of characters **within one segment** (e.g.
    `/api/*/latest.*` matches `/api/front_door/latest.jpg`).
  - `**` matches **any number of segments** (e.g. `/api/**`).

Multiple rules on a key are unioned — the request is allowed if any of them
matches. A key with no rules can access nothing.

## Running

```bash
cargo run -- bosun.example.json
```

Make an authenticated request (allowed by the `viewer` example rule):

```bash
curl -H "X-API-Key: change-me-viewer-key" http://localhost:8080/api/events
```

Health check (no auth required):

```bash
curl http://localhost:8080/healthz   # -> ok
```

Logging verbosity is controlled with `RUST_LOG`, e.g.
`RUST_LOG=bosun=debug cargo run -- bosun.example.json`.

## Home Assistant add-on

This repository is also a Home Assistant **custom add-on repository**. To install
Bosun on Home Assistant:

1. **Settings → Add-ons → Add-on Store → ⋮ → Repositories**, add
   `https://github.com/nsg/bosun`.
2. Install **Bosun**, configure `frigate_url` and `api_keys` in the add-on's
   **Configuration** tab, then start it.

The add-on ([`bosun/config.yaml`](bosun/config.yaml)) runs a prebuilt image and
exposes the same allowlist as native Home Assistant options; the entrypoint
([`docker/run.sh`](docker/run.sh)) translates those options into Bosun's config.

### Releasing

There are no git tags. A release is whatever is on `master`:

- On every push to `master`, [`.github/workflows/build.yml`](.github/workflows/build.yml)
  builds per-architecture images (`amd64`, `aarch64`) and publishes them to
  `ghcr.io/nsg/bosun-<arch>`.
- Home Assistant pulls the image matching the `version` in
  [`bosun/config.yaml`](bosun/config.yaml). To ship an update, bump that
  `version` (and `Cargo.toml`), and push — Home Assistant then offers an
  **Update**.

> One-time setup: after the first successful build, mark the GHCR packages
> `bosun-amd64` and `bosun-aarch64` **public** so Home Assistant can pull them
> without credentials.

## Development

```bash
cargo test      # unit + integration tests
cargo clippy
cargo fmt
```

Tests cover Bosun's own behavior — rule matching, config parsing, and the
access-control + proxy flow against a mock upstream — not third-party crates.
