#!/usr/bin/env python3
"""Check that the root Cargo package matches a validated release version."""

import argparse
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def cargo_package_version(manifest_path: Path) -> str:
    manifest_path = manifest_path.resolve()
    result = subprocess.run(
        [
            "cargo",
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
            str(manifest_path),
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        diagnostic = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"cargo metadata failed: {diagnostic}")

    try:
        metadata = json.loads(result.stdout)
        package = next(
            package
            for package in metadata["packages"]
            if Path(package["manifest_path"]).resolve() == manifest_path
        )
        return package["version"]
    except (json.JSONDecodeError, KeyError, StopIteration) as error:
        raise RuntimeError(
            f"cargo metadata did not contain the package at {manifest_path}"
        ) from error


def check_release_version(release_version: str, manifest_path: Path) -> None:
    package_version = cargo_package_version(manifest_path)
    if package_version != release_version:
        raise RuntimeError(
            "release version mismatch: "
            f"Cargo package version is {package_version}, "
            f"but the validated release version is {release_version}"
        )

    print(f"Cargo package version {package_version} matches the release version")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("release_version", help="validated release version")
    parser.add_argument(
        "--manifest-path",
        type=Path,
        default=ROOT / "Cargo.toml",
        help="Cargo manifest to inspect (default: repository root Cargo.toml)",
    )
    args = parser.parse_args()

    try:
        check_release_version(args.release_version, args.manifest_path)
    except RuntimeError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
