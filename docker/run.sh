#!/bin/sh
# Home Assistant add-on entrypoint. The Supervisor writes the user's add-on
# options to /data/options.json; reshape those into a Bosun config and run it.
# The container always listens on 0.0.0.0:8080 — map it to a host port via the
# add-on's "Network" section.
set -eu

OPTIONS=/data/options.json
CONFIG=/data/bosun.json

if [ -f "$OPTIONS" ]; then
    jq '{
        listen: "0.0.0.0:8080",
        connect_timeout: (.connect_timeout // 10),
        frigate: { url: .frigate_url },
        api_keys: (.api_keys // [])
    }' "$OPTIONS" > "$CONFIG"

    LEVEL=$(jq -r '.log_level // "info"' "$OPTIONS")
    export RUST_LOG="bosun=${LEVEL},tower_http=${LEVEL}"

    exec bosun "$CONFIG"
fi

# Fallback for running the image outside Home Assistant: use a config passed as
# an argument, or the bundled example.
exec bosun "${1:-/usr/share/bosun/bosun.example.json}"
