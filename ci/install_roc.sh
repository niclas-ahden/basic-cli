#!/usr/bin/env bash
# Install a PINNED roc nightly (new compiler) from roc-lang/nightlies.
#
# We pin a specific nightly (rather than tracking "latest") because the new
# compiler's host ABI still changes day-to-day; the committed glue
# (src/roc_platform_abi.rs) is generated for exactly this compiler, so CI must
# use the same one. To adopt a newer compiler: regenerate the glue against it,
# reconcile src/lib.rs if the generated helpers changed, and update .roc-version
# to the new commit — all in one deliberate change.
#
# Single source of truth: .roc-version holds the roc commit SHA. We resolve the
# matching roc-lang/nightlies release (its tag ends with the short SHA) and
# install the asset for this platform, appending its bin dir to PATH/GITHUB_PATH.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMMIT="$(tr -d '[:space:]' < "$ROOT_DIR/.roc-version")"
SHORT="${COMMIT:0:7}"

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

echo "install_roc: resolving roc-lang/nightlies release for commit ${SHORT}..."
TAG="$(curl -fsSL "${AUTH[@]}" "${API}?per_page=100" \
    | grep -oE "\"tag_name\": *\"nightly-[^\"]*-${SHORT}\"" \
    | head -1 | sed -E 's/.*"(nightly-[^"]*)".*/\1/')"

if [ -z "${TAG:-}" ]; then
    echo "install_roc: no nightlies release found whose tag ends in -${SHORT}." >&2
    echo "Pin .roc-version to a commit that has a published nightly." >&2
    exit 1
fi

URL="$(curl -fsSL "${AUTH[@]}" "${API}/tags/${TAG}" \
    | grep -oE "https://github.com/roc-lang/nightlies/releases/download/${TAG}/roc_nightly-${PLATFORM}-[^\"]*\.tar\.gz" \
    | head -1)"

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
