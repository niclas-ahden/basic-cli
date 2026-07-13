$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RustTarget = "x86_64-pc-windows-msvc"
$RocTarget = "x64win"
$OutputDir = Join-Path $RootDir "platform/targets/$RocTarget"
$HostLibrary = Join-Path $RootDir "target/$RustTarget/release/host.lib"
$WindowsSdkLibRoot = Join-Path ${env:ProgramFiles(x86)} "Windows Kits/10/Lib"
$SystemLibraries = @(
    "advapi32.lib",
    "bcrypt.lib",
    "crypt32.lib",
    "dbghelp.lib",
    "iphlpapi.lib",
    "kernel32.lib",
    "ncrypt.lib",
    "ntdll.lib",
    "ole32.lib",
    "secur32.lib",
    "shell32.lib",
    "user32.lib",
    "userenv.lib",
    "ws2_32.lib"
)

Write-Host "Building for $RocTarget ($RustTarget)..."
rustup target add $RustTarget
if ($LASTEXITCODE -ne 0) {
    throw "rustup target add failed with exit code $LASTEXITCODE"
}
cargo build --locked --release --lib --target $RustTarget
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
Copy-Item -Force $HostLibrary (Join-Path $OutputDir "host.lib")
Write-Host "  -> platform/targets/$RocTarget/host.lib"

# Rust static libraries do not embed their Windows SDK dependencies. Include
# the import libraries in the Roc package so linking works from a downloaded
# bundle, including outside a Visual Studio developer shell.
$WindowsSdkVersion = Get-ChildItem -Path $WindowsSdkLibRoot -Directory |
    Where-Object { Test-Path (Join-Path $_.FullName "um/x64/ws2_32.lib") } |
    Sort-Object Name -Descending |
    Select-Object -First 1

if ($null -eq $WindowsSdkVersion) {
    throw "Could not find x64 Windows SDK libraries under $WindowsSdkLibRoot"
}

foreach ($Library in $SystemLibraries) {
    $Source = Join-Path $WindowsSdkVersion.FullName "um/x64/$Library"
    if (-not (Test-Path $Source)) {
        throw "Could not find required Windows SDK library: $Source"
    }
    Copy-Item -Force $Source (Join-Path $OutputDir $Library)
    Write-Host "  -> platform/targets/$RocTarget/$Library"
}
