#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$ROOT_DIR"

EXAMPLE_NAMES=()
TEST_NAMES=()
TEST_EXPECT_NAMES=(
    "env"
    "file"
    "path-test"
    "sqlite"
    "tcp"
    "url"
    "utc"
)

SKIPPED_EXPECT_NAMES=(
    # roc build tests/cmd-test.roc currently segfaults in the compiler.
    # Tracked upstream: https://github.com/roc-lang/roc/issues/10003
    "cmd-test"
)

EXPECT_HELPER_NAMES=(
    "shared-code"
)

name_in_array() {
    local needle="$1"
    shift

    local item
    for item in "$@"; do
        if [ "$item" = "$needle" ]; then
            return 0
        fi
    done

    return 1
}

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
    for example in "${EXAMPLE_NAMES[@]}"; do
        rm -f "examples/${example}"
    done

    for test in "${TEST_NAMES[@]}"; do
        rm -f "tests/${test}"
    done

    # Remove temporary databases created by expect tests.
    rm -f examples/*.e2e.db
    rm -f tests/*.e2e.db

    # Remove bundle file
    if [ -n "${BUNDLE_FILE:-}" ] && [ -f "$BUNDLE_FILE" ]; then
        rm -f "$BUNDLE_FILE"
    fi
}

# Set up trap to ensure cleanup runs on exit
trap cleanup EXIT

echo "=== basic-cli CI ==="
echo ""

# Roc is provided on PATH by roc-lang/setup-roc in CI. For local runs any `roc`
# on PATH is used.
if ! command -v roc &>/dev/null; then
    echo "Error: 'roc' was not found on PATH." >&2
    echo "Install a recent roc binary and put it on PATH." >&2
    exit 1
fi

echo "Using roc version: $(roc version)"

if [ "$(uname -s)" = "Darwin" ] && [ -z "${SDKROOT:-}" ]; then
    SDKROOT=$(xcrun --sdk macosx --show-sdk-path 2>/dev/null || true)
    if [ -n "$SDKROOT" ]; then
        export SDKROOT
        echo "Using SDKROOT: $SDKROOT"
    fi
fi

# NOTE: the committed src/roc_platform_abi.rs (generated glue) is the source of
# truth for the host ABI and is intentionally NOT re-checked here: CI installs
# only the roc binary (no glue spec to regenerate from). Correctness is enforced
# by the host compiling against committed glue and the end-to-end expect tests
# below exercising the real ABI. If a new nightly changes generated glue, run
# ci/regenerate_glue.sh locally, reconcile src/lib.rs, and commit together.

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

for roc_file in "${EXAMPLES_DIR}"*.roc; do
    [ -f "$roc_file" ] && EXAMPLE_NAMES+=("$(basename "${roc_file%.roc}")")
done

MISSING_EXPECT=0
for example in "${EXAMPLE_NAMES[@]}"; do
    if [ ! -f "ci/expect_scripts/${example}.exp" ]; then
        echo "Error: missing expect script for examples/${example}.roc" >&2
        MISSING_EXPECT=1
    fi
done
if [ "$MISSING_EXPECT" -ne 0 ]; then
    exit 1
fi

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

echo ""
echo "=== Testing examples ==="
for example in "${EXAMPLE_NAMES[@]}"; do
    echo "Testing: ${example}.roc"
    roc test "examples/${example}.roc"
done

for roc_file in "${TESTS_DIR}"*.roc; do
    [ -f "$roc_file" ] && TEST_NAMES+=("$(basename "${roc_file%.roc}")")
done

for test in "${TEST_EXPECT_NAMES[@]}"; do
    if [ ! -f "tests/${test}.roc" ]; then
        echo "Error: missing tests/${test}.roc for expect test" >&2
        exit 1
    fi
    if [ ! -f "ci/expect_scripts/${test}.exp" ]; then
        echo "Error: missing expect script for tests/${test}.roc" >&2
        exit 1
    fi
done

for test in "${SKIPPED_EXPECT_NAMES[@]}"; do
    if [ ! -f "tests/${test}.roc" ]; then
        echo "Error: skipped expect test tests/${test}.roc does not exist" >&2
        exit 1
    fi
    if [ ! -f "ci/expect_scripts/${test}.exp" ]; then
        echo "Error: skipped expect script ci/expect_scripts/${test}.exp does not exist" >&2
        exit 1
    fi
done

UNTRACKED_EXPECT=0
for expect_file in ci/expect_scripts/*.exp; do
    expect_name="$(basename "${expect_file%.exp}")"
    if name_in_array "$expect_name" "${EXAMPLE_NAMES[@]}"; then
        continue
    fi
    if name_in_array "$expect_name" "${TEST_EXPECT_NAMES[@]}"; then
        continue
    fi
    if name_in_array "$expect_name" "${SKIPPED_EXPECT_NAMES[@]}"; then
        continue
    fi
    if name_in_array "$expect_name" "${EXPECT_HELPER_NAMES[@]}"; then
        continue
    fi

    echo "Error: ci/expect_scripts/${expect_name}.exp is not active or explicitly skipped" >&2
    UNTRACKED_EXPECT=1
done
if [ "$UNTRACKED_EXPECT" -ne 0 ]; then
    exit 1
fi

echo ""
echo "=== Checking tests ==="
for test in "${TEST_NAMES[@]}"; do
    if name_in_array "$test" "${SKIPPED_EXPECT_NAMES[@]}"; then
        echo "Skipping check: ${test}.roc (known upstream blocker)"
        continue
    fi

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

if [ "${#TEST_EXPECT_NAMES[@]}" -gt 0 ]; then
    echo ""
    echo "=== Building standalone expect tests ==="
fi
for test in "${TEST_EXPECT_NAMES[@]}"; do
    echo "Building: ${test}.roc"
    roc build "tests/${test}.roc"
    mv "./${test}" "tests/"
done

# The HTTP expect tests drive a local HTTP server; build it up front.
if printf '%s\n' "${EXAMPLE_NAMES[@]}" | grep -Eqx "http|http-client"; then
    echo ""
    echo "=== Building HTTP test server ==="
    (cd ci/rust_http_server && cargo build --release)
fi

# Run each example's expect test. Every shipped example is expected to have one.
echo ""
echo "=== Running example expect tests ==="
FAILED=0
for example in "${EXAMPLE_NAMES[@]}"; do
    echo ""
    echo "--- Testing: $example ---"
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
echo "=== Running standalone expect tests ==="
for test in "${TEST_EXPECT_NAMES[@]}"; do
    echo ""
    echo "--- Testing: $test ---"
    set +e
    expect "ci/expect_scripts/${test}.exp"
    EXIT_CODE=$?
    set -e
    if [ $EXIT_CODE -eq 0 ]; then
        echo "PASS: $test"
    else
        echo "FAIL: $test (exit code: $EXIT_CODE)"
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
