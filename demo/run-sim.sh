#!/usr/bin/env bash
# Start the simulator container (runs esp32_sim as default CMD).
# Frontend will be reachable at http://127.0.0.1:8080
set -euo pipefail

cd "$(dirname "$0")"

# Pass env from .env if present (so MINIMAX_API_KEY etc. reach the container).
if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

echo "Starting ESP32 Smart Room demo container..."
echo "  - Simulator + WebSocket frontend on http://127.0.0.1:8080"
echo "  - Use ./demo/run-zeroclaw.sh to talk to the agent"
echo

exec docker compose up
