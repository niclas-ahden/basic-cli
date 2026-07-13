$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RustTarget = "x86_64-pc-windows-msvc"
$RocTarget = "x64win"
$OutputDir = Join-Path $RootDir "platform/targets/$RocTarget"
$HostLibrary = Join-Path $RootDir "target/$RustTarget/release/host.lib"

Write-Host "Building for $RocTarget ($RustTarget)..."
rustup target add $RustTarget
if ($LASTEXITCODE -ne 0) {
    throw "rustup target add failed with exit code $LASTEXITCODE"
}
cargo build --release --lib --target $RustTarget
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
Copy-Item -Force $HostLibrary (Join-Path $OutputDir "host.lib")
Write-Host "  -> platform/targets/$RocTarget/host.lib"
