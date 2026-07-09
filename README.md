# Bosun

An API gateway that sits between clients and a [Frigate](https://frigate.video)
NVR instance.

> **Status:** scaffolding. Right now Bosun boots an HTTP server, reads a minimal
> config, and reverse-proxies every request to Frigate. Authentication and
> access control are not implemented yet.

## How it works

```
client ──▶ Bosun ──▶ Frigate
```

Requests are streamed through to the configured Frigate upstream and the
response is streamed back unchanged.

## Configuration

Bosun reads a TOML file (default `bosun.toml`, or pass a path as the first
argument). See [`bosun.example.toml`](bosun.example.toml).

```toml
listen = "0.0.0.0:8080"

[frigate]
url = "http://localhost:5000"
```

## Running

```bash
cargo run -- bosun.toml
```

Health check:

```bash
curl http://localhost:8080/healthz   # -> ok
```

Logging verbosity is controlled with `RUST_LOG`, e.g.
`RUST_LOG=bosun=debug cargo run -- bosun.toml`.

## Development

```bash
cargo build
cargo clippy
cargo fmt
```
