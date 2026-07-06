#!/usr/bin/env bash
# Install the current Roc nightly from roc-lang/nightlies.
#
# CI intentionally tracks the latest nightly compiler. The committed Rust glue is
# still the host ABI source of truth; if a new nightly changes generated glue,
# regenerate src/roc_platform_abi.rs and reconcile src/lib.rs in the same change.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)            PLATFORM="linux_x86_64" ;;
    Linux-aarch64|Linux-arm64) PLATFORM="linux_arm64" ;;
    Darwin-x86_64)           PLATFORM="macos_x86_64" ;;
    Darwin-arm64)            PLATFORM="macos_apple_silicon" ;;
    *) echo "install_roc: unsupported platform $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

# Authenticated requests avoid GitHub's low anonymous API rate limit in CI.
AUTH=()
if [ -n "${GITHUB_TOKEN:-}" ]; then
    AUTH=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
fi
API="https://api.github.com/repos/roc-lang/nightlies/releases"

echo "install_roc: resolving latest roc-lang/nightlies release..."
TAG="$(curl -fsSL "${AUTH[@]}" "${API}/latest" \
    | grep -oE "\"tag_name\": *\"nightly-[^\"]+\"" \
    | head -1 | sed -E 's/.*"(nightly-[^"]*)".*/\1/' || true)"

if [ -z "${TAG:-}" ]; then
    echo "install_roc: could not resolve latest roc-lang/nightlies release." >&2
    exit 1
fi

URL="$(curl -fsSL "${AUTH[@]}" "${API}/tags/${TAG}" \
    | grep -oE "https://github.com/roc-lang/nightlies/releases/download/${TAG}/roc_nightly-${PLATFORM}-[^\"]*\.tar\.gz" \
    | head -1 || true)"

if [ -z "${URL:-}" ]; then
    echo "install_roc: release ${TAG} has no ${PLATFORM} asset." >&2
    exit 1
fi

DEST="$ROOT_DIR/.roc-bin"
rm -rf "$DEST" && mkdir -p "$DEST"
echo "install_roc: downloading ${URL}"
curl -fsSL "$URL" | tar -xz -C "$DEST"

BIN_DIR="$(dirname "$(find "$DEST" -type f -name roc | head -1)")"
if [ -z "${BIN_DIR:-}" ]; then
    echo "install_roc: roc binary not found in downloaded archive." >&2
    exit 1
fi

export PATH="$BIN_DIR:$PATH"
if [ -n "${GITHUB_PATH:-}" ]; then
    echo "$BIN_DIR" >> "$GITHUB_PATH"
fi

echo "install_roc: installed $(roc version)"
