#!/usr/bin/env python3
from __future__ import annotations

import argparse
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PLATFORM_DIR = ROOT / "platform"
LIBRARY_EXTENSIONS = {".a", ".o", ".lib", ".obj"}


def relative_platform_path(path: Path) -> str:
    return path.relative_to(PLATFORM_DIR).as_posix()


def main() -> None:
    parser = argparse.ArgumentParser(description="Bundle the basic-cli platform")
    parser.add_argument("--output-dir", type=Path, default=ROOT)
    args, roc_args = parser.parse_known_args()

    output_dir = args.output_dir
    if not output_dir.is_absolute():
        output_dir = ROOT / output_dir
    output_dir.mkdir(parents=True, exist_ok=True)
    output_dir = output_dir.resolve()

    roc_files = sorted(PLATFORM_DIR.glob("*.roc"))
    library_files = sorted(
        path
        for path in (PLATFORM_DIR / "targets").rglob("*")
        if path.is_file() and path.suffix in LIBRARY_EXTENSIONS
    )
    bundle_files = [
        *(relative_platform_path(path) for path in roc_files),
        *(relative_platform_path(path) for path in library_files),
    ]

    print(
        f"Bundling {len(roc_files)} .roc files and "
        f"{len(library_files)} library files...\n"
    )
    print("Files to bundle:")
    for path in bundle_files:
        print(f"  {path}")
    print("  THIRD_PARTY_LICENSES.md\n", flush=True)

    license_target = PLATFORM_DIR / "THIRD_PARTY_LICENSES.md"
    shutil.copy2(ROOT / "THIRD_PARTY_LICENSES.md", license_target)
    try:
        subprocess.run(
            [
                "roc",
                "bundle",
                *bundle_files,
                "THIRD_PARTY_LICENSES.md",
                "--output-dir",
                str(output_dir),
                *roc_args,
            ],
            cwd=PLATFORM_DIR,
            check=True,
        )
    finally:
        license_target.unlink(missing_ok=True)


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        raise SystemExit(error.returncode) from None
