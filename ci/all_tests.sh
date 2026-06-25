#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$ROOT_DIR"

EXAMPLE_NAMES=()

# Cleanup function to restore examples and stop HTTP server
cleanup() {
    echo ""
    echo "=== Cleaning up ==="

    # Restore examples from backups
    for f in examples/*.roc.bak; do
        if [ -f "$f" ]; then
            mv "$f" "${f%.bak}"
        fi
    done

    # Stop HTTP server if running
    if [ -n "${HTTP_SERVER_PID:-}" ]; then
        kill "$HTTP_SERVER_PID" 2>/dev/null || true
    fi

    # Remove built binaries
    for example in "${EXAMPLE_NAMES[@]:-}"; do
        rm -f "examples/${example}"
    done

    # Remove bundle file
    if [ -n "${BUNDLE_FILE:-}" ] && [ -f "$BUNDLE_FILE" ]; then
        rm -f "$BUNDLE_FILE"
    fi
}

# Set up trap to ensure cleanup runs on exit
trap cleanup EXIT

# Get the roc commit pinned in .roc-version
ROC_COMMIT=$(python3 ci/get_roc_commit.py)
ROC_COMMIT_SHORT="${ROC_COMMIT:0:8}"
NEED_BUILD=true
USE_ROC_SRC=false

echo "=== basic-cli CI ==="
echo ""

# Check if roc is already on PATH and matches pinned commit
if command -v roc &>/dev/null; then
    SYSTEM_VERSION=$(roc version 2>/dev/null || echo "unknown")
    if echo "$SYSTEM_VERSION" | grep -q "$ROC_COMMIT_SHORT"; then
        echo "roc on PATH matches pinned commit: $SYSTEM_VERSION"
        NEED_BUILD=false
    else
        echo "roc on PATH ($SYSTEM_VERSION) doesn't match pinned commit ($ROC_COMMIT_SHORT)"
    fi
fi

# Check cached build in roc-src/
if [ "$NEED_BUILD" = true ] && [ -d "roc-src" ] && [ -f "roc-src/zig-out/bin/roc" ]; then
    CACHED_VERSION=$(./roc-src/zig-out/bin/roc version 2>/dev/null || echo "unknown")
    if echo "$CACHED_VERSION" | grep -q "$ROC_COMMIT_SHORT"; then
        echo "roc in roc-src/ matches pinned commit: $CACHED_VERSION"
        NEED_BUILD=false
        USE_ROC_SRC=true
    else
        echo "Cached roc ($CACHED_VERSION) doesn't match pinned commit ($ROC_COMMIT_SHORT)"
        echo "Removing stale roc-src..."
        rm -rf roc-src
    fi
fi

if [ "$NEED_BUILD" = true ]; then
    ROC_COMMIT="$ROC_COMMIT" ./ci/build_pinned_roc.sh
    USE_ROC_SRC=true
fi

# Prefer the cached/source-built roc if it exists; otherwise keep the matching PATH roc.
if [ "$USE_ROC_SRC" = true ]; then
    export PATH="$(pwd)/roc-src/zig-out/bin:$PATH"
fi

echo ""
echo "Using roc version: $(roc version)"

if [ "$(uname -s)" = "Darwin" ] && [ -z "${SDKROOT:-}" ]; then
    SDKROOT=$(xcrun --sdk macosx --show-sdk-path 2>/dev/null || true)
    if [ -n "$SDKROOT" ]; then
        export SDKROOT
        echo "Using SDKROOT: $SDKROOT"
    fi
fi

echo ""
echo "=== Checking generated Rust glue ==="
./ci/regenerate_glue.sh --check

# Build the platform
if [ "${NO_BUILD:-}" != "1" ]; then
    echo ""
    echo "=== Building platform ==="
    ./build.sh
else
    echo ""
    echo "=== Skipping platform build (NO_BUILD=1) ==="
fi

EXAMPLES_DIR="${ROOT_DIR}/examples/"
export EXAMPLES_DIR

TESTS_DIR="${ROOT_DIR}/tests/"
export TESTS_DIR

# Examples with maintained expect tests. All examples are checked and built
# below; this list only controls which built binaries are executed.
EXPECT_EXAMPLES=(
    "command-line-args"
    "hello-world"
    "stdin-basic"
    "path"
    "command"
    "file-read-write"
    "time"
    "random"
    "locale"
    "tty"
    "dir"
    "env-var"
)

for roc_file in "${EXAMPLES_DIR}"*.roc; do
    [ -f "$roc_file" ] && EXAMPLE_NAMES+=("$(basename "${roc_file%.roc}")")
done

# Check if all target libraries exist for bundling
ALL_TARGETS_EXIST=true
for target in x64mac arm64mac x64musl arm64musl; do
    if [ ! -f "platform/targets/$target/libhost.a" ]; then
        ALL_TARGETS_EXIST=false
        break
    fi
done

# Bundle and set up HTTP server if all targets exist
BUNDLE_FILE=""
HTTP_SERVER_PID=""
USE_BUNDLE=false

if [ "${NO_BUNDLE:-}" = "1" ]; then
    echo ""
    echo "=== Skipping bundle (NO_BUNDLE=1) ==="
elif [ "$ALL_TARGETS_EXIST" = true ]; then
    echo ""
    echo "=== Bundling platform ==="
    BUNDLE_OUTPUT=$(./bundle.sh 2>&1)
    echo "$BUNDLE_OUTPUT"

    # Extract bundle filename from output
    BUNDLE_PATH=$(echo "$BUNDLE_OUTPUT" | grep "^Created:" | awk '{print $2}')
    BUNDLE_FILE=$(basename "$BUNDLE_PATH")

    if [ -n "$BUNDLE_FILE" ] && [ -f "$BUNDLE_FILE" ]; then
        echo ""
        echo "=== Starting HTTP server for bundle testing ==="
        python3 -m http.server 8000 &
        HTTP_SERVER_PID=$!
        sleep 2

        # Verify server is running
        if curl -f -I "http://localhost:8000/$BUNDLE_FILE" > /dev/null 2>&1; then
            echo "HTTP server running at http://localhost:8000"
            echo "Bundle: $BUNDLE_FILE"

            # Modify examples to use bundle URL
            echo ""
            echo "=== Configuring examples to use bundle ==="
            for example in examples/*.roc; do
                sed -i.bak "s|platform \"../platform/main.roc\"|platform \"http://localhost:8000/$BUNDLE_FILE\"|" "$example"
            done
            USE_BUNDLE=true
        else
            echo "Warning: HTTP server failed to start, testing with local platform"
            kill "$HTTP_SERVER_PID" 2>/dev/null || true
            HTTP_SERVER_PID=""
        fi
    else
        echo "Warning: Bundle creation failed, testing with local platform"
    fi
else
    echo ""
    echo "=== Skipping bundle (not all targets built) ==="
    echo "Run './build.sh --all' first to test with bundled platform"
fi

echo ""
echo "=== Checking examples ==="
for example in "${EXAMPLE_NAMES[@]}"; do
    echo "Checking: ${example}.roc"
    roc check "examples/${example}.roc"
done

TESTS_FILES=()
for roc_file in "${TESTS_DIR}"*.roc; do
    [ -f "$roc_file" ] && TESTS_FILES+=("$(basename "${roc_file%.roc}")")
done

echo ""
echo "=== Checking tests ==="
for test in "${TESTS_FILES[@]}"; do
    echo "Checking: ${test}.roc"
    roc check "tests/${test}.roc"
done

echo ""
if [ "$USE_BUNDLE" = true ]; then
    echo "=== Building examples (using bundle) ==="
else
    echo "=== Building examples (using local platform) ==="
fi
for example in "${EXAMPLE_NAMES[@]}"; do
    echo "Building: ${example}.roc"
    roc build "examples/${example}.roc"
    mv "./${example}" "examples/"
done

# Run expect tests
echo ""
echo "=== Running expect tests ==="
FAILED=0
for example in "${EXPECT_EXAMPLES[@]}"; do
    echo ""
    echo "--- Testing: $example ---"
    if [ ! -f "ci/expect_scripts/${example}.exp" ]; then
        echo "FAIL: missing expect script for $example"
        FAILED=1
        continue
    fi
    set +e
    expect "ci/expect_scripts/${example}.exp"
    EXIT_CODE=$?
    set -e
    if [ $EXIT_CODE -eq 0 ]; then
        echo "PASS: $example"
    else
        echo "FAIL: $example (exit code: $EXIT_CODE)"
        FAILED=1
    fi
done

echo ""
if [ $FAILED -eq 0 ]; then
    if [ "$USE_BUNDLE" = true ]; then
        echo "=== All tests passed (with bundle)! ==="
    else
        echo "=== All tests passed! ==="
    fi
else
    echo "=== Some tests failed ==="
    exit 1
fi
