#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ROC_COMMIT="${ROC_COMMIT:-$(python3 ci/get_roc_commit.py)}"
MAX_ATTEMPTS="${ROC_BUILD_RETRY_COUNT:-4}"
TRANSIENT_BUILD_ERROR='error: (unable|invalid HTTP response)|HttpConnectionClosing'

echo "Building roc from pinned commit $ROC_COMMIT..."

rm -rf roc-src
git init roc-src
cd roc-src
git remote add origin https://github.com/roc-lang/roc
git fetch --depth 1 origin "$ROC_COMMIT"
git checkout --detach "$ROC_COMMIT"

attempt=1
while [ "$attempt" -le "$MAX_ATTEMPTS" ]; do
    echo "Building roc attempt $attempt of $MAX_ATTEMPTS"
    output_file="$(mktemp)"

    set +e
    zig build roc > "$output_file" 2>&1
    status=$?
    set -e

    cat "$output_file"

    if [ "$status" -eq 0 ]; then
        rm -f "$output_file"
        break
    fi

    if [ "$attempt" -ge "$MAX_ATTEMPTS" ]; then
        rm -f "$output_file"
        exit "$status"
    fi

    if grep -Eq "$TRANSIENT_BUILD_ERROR" "$output_file"; then
        echo "Transient roc build failure; retrying..."
        rm -f "$output_file"
        attempt=$((attempt + 1))
        sleep 2
    else
        rm -f "$output_file"
        exit "$status"
    fi
done

if [ -n "${GITHUB_PATH:-}" ]; then
    echo "$(pwd)/zig-out/bin" >> "$GITHUB_PATH"
fi
