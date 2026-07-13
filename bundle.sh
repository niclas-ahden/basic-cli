#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "$0")" && pwd)"
output_dir="$root_dir"
args=()

while (($# > 0)); do
    case "$1" in
        --output-dir)
            if (($# < 2)); then
                echo "Error: --output-dir requires a value" >&2
                exit 2
            fi
            output_dir="$2"
            shift 2
            ;;
        --output-dir=*)
            output_dir="${1#--output-dir=}"
            shift
            ;;
        *)
            args+=("$1")
            shift
            ;;
    esac
done

if [[ "$output_dir" != /* ]]; then
    output_dir="$root_dir/$output_dir"
fi
mkdir -p "$output_dir"
output_dir="$(cd "$output_dir" && pwd)"

cd "$root_dir/platform"

# Collect all .roc files
roc_files=(*.roc)

# Collect all host libraries and runtime files from targets directories
lib_files=()
for lib in targets/*/*.a targets/*/*.o; do
    if [[ -f "$lib" ]]; then
        lib_files+=("$lib")
    fi
done

echo "Bundling ${#roc_files[@]} .roc files and ${#lib_files[@]} library files..."
echo ""
echo "Files to bundle:"
for f in "${roc_files[@]}"; do
    echo "  $f"
done
for f in "${lib_files[@]}"; do
    echo "  $f"
done
echo "  THIRD_PARTY_LICENSES.md"
echo ""

# Copy THIRD_PARTY_LICENSES.md into platform dir (roc bundle doesn't allow .. paths)
cp "$root_dir/THIRD_PARTY_LICENSES.md" .
trap 'rm -f THIRD_PARTY_LICENSES.md' EXIT

if ((${#args[@]} > 0)); then
    roc bundle "${roc_files[@]}" "${lib_files[@]}" THIRD_PARTY_LICENSES.md \
        --output-dir "$output_dir" "${args[@]}"
else
    roc bundle "${roc_files[@]}" "${lib_files[@]}" THIRD_PARTY_LICENSES.md \
        --output-dir "$output_dir"
fi
