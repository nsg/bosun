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

Everything is configured from the add-on's **Configuration** tab — no config
files to edit. The form gives you the right control for each field:

- **Frigate URL** — text field for Frigate's base URL. If Frigate runs as its
  own add-on, this is typically `http://<frigate-slug>:5000` or
  `http://homeassistant.local:5000`.
- **Upstream connect timeout** — a slider (1–60 s) bounding how long Bosun waits
  to connect to Frigate. It bounds connection setup only, so it never cuts off
  snapshots or video streams.
- **Log level** — a dropdown: `trace`, `debug`, `info`, `warn`, `error`.
- **API keys** — an editable list, one entry per caller. Use **＋** to add a
  key. Each key has:
  - **name** — a label for the log.
  - **key** — the secret clients send in the `X-API-Key` header (shown as a
    masked password field).
  - **rules** — an editable list of allow rules. Each rule has a set of
    **methods** (dropdowns: `GET`, `HEAD`, `POST`, …, or `*` for any) and a set
    of **paths** (glob patterns).

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
