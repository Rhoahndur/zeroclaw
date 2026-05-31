#!/usr/bin/env bash
# Low-storage / MacBook Air friendly path.
# Runs the ESP32 simulator + visualizer directly on your host (no Docker required for the sim).
#
# This is the recommended way to test the vignette when you don't have 60-80+ GB free for Docker Desktop.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "=== ZeroClaw ESP32 Smart Room – Host (low storage) mode ==="
echo

if ! command -v socat >/dev/null 2>&1; then
  echo "socat is required. Install with: brew install socat"
  exit 1
fi

# Make sure a config exists for the agent later
mkdir -p demo/data/config
cp -n demo/zeroclaw.toml.example demo/data/config/config.toml 2>/dev/null || true

echo "Starting esp32_sim + visualizer directly on host..."
echo "  Frontend will be at http://127.0.0.1:8080"
echo "  In another terminal run the agent with:"
echo "    ./demo/run-agent-host.sh"
echo
echo "Then paste the primer from demo/PROMPTS.md and try natural language."
echo

exec cargo run -p zeroclaw-hardware --example esp32_sim --features "hardware dev-sim" "$@"
