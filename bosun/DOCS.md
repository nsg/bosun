# Bosun

Bosun is an API gateway that sits in front of a [Frigate](https://frigate.video)
NVR. It authenticates callers by API key and enforces a **default-deny
allowlist** over which HTTP method + path each key may reach, then
reverse-proxies permitted requests to Frigate.

## Installation

1. In Home Assistant, go to **Settings → Add-ons → Add-on Store**.
2. Open the **⋮** menu (top right) → **Repositories** and add:
   `https://github.com/nsg/bosun`
3. Install the **Bosun** add-on from the store, configure it (below), and start it.

Updates are delivered through Home Assistant: when a new version is published,
the add-on shows an **Update** button.

## Configuration

Example options:

```yaml
frigate_url: "http://homeassistant.local:5000"
log_level: info
api_keys:
  - name: viewer
    key: a-long-random-secret
    rules:
      - methods:
          - GET
          - HEAD
        paths:
          - /api/events
          - /api/events/*
          - /api/*/latest.*
```

### Options

- **`frigate_url`** — Base URL of the Frigate instance to proxy to. If Frigate
  runs as its own add-on, this is typically `http://<frigate-slug>:5000` or
  `http://homeassistant.local:5000`.
- **`log_level`** — One of `trace`, `debug`, `info`, `warn`, `error`.
- **`api_keys`** — One entry per caller. Each has a `name`, a secret `key`
  (sent by clients in the `X-API-Key` header), and a list of `rules`.

### Rules (default-deny)

A request is allowed only when one of the key's rules matches **both** the HTTP
method and the request path. Anything not explicitly allowed is rejected — `401`
if the key is unknown, `403` if the key exists but no rule matches.

- **`methods`** — HTTP verbs granted, case-insensitive. `"*"` matches any verb.
- **`paths`** — glob patterns: `*` matches any run of characters within a single
  path segment (e.g. `/api/*/latest.*` → `/api/front_door/latest.jpg`); `**`
  matches any number of segments (e.g. `/api/**`).

## Networking

The add-on listens on port `8080` inside the container. Map it to a host port in
the add-on's **Network** section. Health checks are available (no key required)
at `/healthz`.

Make an authenticated request:

```bash
curl -H "X-API-Key: a-long-random-secret" http://<home-assistant>:8080/api/events
```
