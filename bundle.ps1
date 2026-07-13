param(
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$PlatformDir = Join-Path $RootDir "platform"
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = $RootDir
}
elseif (-not [System.IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path $RootDir $OutputDir
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$OutputDir = (Resolve-Path $OutputDir).Path

$RocFiles = Get-ChildItem $PlatformDir -Filter *.roc |
    Sort-Object Name |
    ForEach-Object { $_.Name }
$LibraryFiles = Get-ChildItem (Join-Path $PlatformDir "targets") -Recurse -File |
    Where-Object { $_.Extension -in ".a", ".o", ".lib", ".obj" } |
    Sort-Object FullName |
    ForEach-Object { [System.IO.Path]::GetRelativePath($PlatformDir, $_.FullName) }

$LicenseSource = Join-Path $RootDir "THIRD_PARTY_LICENSES.md"
$LicenseTarget = Join-Path $PlatformDir "THIRD_PARTY_LICENSES.md"
Copy-Item -Force $LicenseSource $LicenseTarget

Push-Location $PlatformDir
try {
    $BundleArgs = @($RocFiles) + @($LibraryFiles) + @(
        "THIRD_PARTY_LICENSES.md",
        "--output-dir", $OutputDir
    )
    & roc bundle @BundleArgs
    if ($LASTEXITCODE -ne 0) {
        throw "roc bundle failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
    Remove-Item -Force $LicenseTarget -ErrorAction SilentlyContinue
}
