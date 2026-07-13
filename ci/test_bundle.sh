#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 BUNDLE.tar.zst" >&2
    exit 2
fi

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bundle_input="$1"
bundle_dir="$(cd "$(dirname "$bundle_input")" && pwd)"
bundle_file="$(basename "$bundle_input")"
bundle_path="$bundle_dir/$bundle_file"

if [ ! -f "$bundle_path" ]; then
    echo "Bundle does not exist: $bundle_path" >&2
    exit 1
fi

server_pid=""

cleanup() {
    if [ -n "$server_pid" ]; then
        kill "$server_pid" 2>/dev/null || true
    fi
}
trap cleanup EXIT

port="$(python3 - <<'PY'
import socket

with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
)"

python3 -m http.server "$port" --bind 127.0.0.1 --directory "$bundle_dir" &
server_pid=$!
bundle_url="http://127.0.0.1:$port/$bundle_file"

for _ in {1..20}; do
    if curl --fail --silent --head "$bundle_url" >/dev/null; then
        break
    fi
    sleep 0.25
done
if ! curl --fail --silent --head "$bundle_url" >/dev/null; then
    echo "Bundle server did not become ready: $bundle_url" >&2
    exit 1
fi

echo "Testing all examples and standalone tests against $bundle_url"
cd "$root_dir"
NO_BUILD=1 BUNDLE_URL="$bundle_url" ./ci/all_tests.sh
