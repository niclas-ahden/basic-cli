param(
    [string]$BundlePath = ""
)

$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("basic-cli-windows-" + [System.Guid]::NewGuid())
$BackupDir = Join-Path $TempDir "backup"
$BuildDir = Join-Path $TempDir "build"
$Server = $null

function Invoke-Checked {
    param(
        [string]$Command,
        [string[]]$Arguments
    )

    Write-Host "+ $Command $($Arguments -join ' ')"
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command failed with exit code $LASTEXITCODE"
    }
}

function Get-FreePort {
    $Listener = [System.Net.Sockets.TcpListener]::new(
        [System.Net.IPAddress]::Loopback,
        0
    )
    $Listener.Start()
    $Port = $Listener.LocalEndpoint.Port
    $Listener.Stop()
    return $Port
}

New-Item -ItemType Directory -Force -Path $BackupDir, $BuildDir | Out-Null
Copy-Item -Recurse (Join-Path $RootDir "examples") $BackupDir
Copy-Item -Recurse (Join-Path $RootDir "tests") $BackupDir

Push-Location $RootDir
try {
    if ([string]::IsNullOrWhiteSpace($BundlePath)) {
        & (Join-Path $RootDir "build.ps1")
        if ($LASTEXITCODE -ne 0) {
            throw "build.ps1 failed with exit code $LASTEXITCODE"
        }

        $BundleOutput = & (Join-Path $RootDir "bundle.ps1") 2>&1
        if ($LASTEXITCODE -ne 0) {
            $BundleOutput | Write-Host
            throw "bundle.ps1 failed with exit code $LASTEXITCODE"
        }
        $CreatedPath = $null
        foreach ($Line in $BundleOutput) {
            if ($Line -match '^Created:\s+(.+\.tar\.zst)\s*$') {
                $CreatedPath = $Matches[1]
            }
        }
        if (-not $CreatedPath) {
            throw "Could not find the created bundle path"
        }
        $BundlePath = $CreatedPath
    }

    $BundlePath = (Resolve-Path $BundlePath).Path
    $BundleDirectory = Split-Path -Parent $BundlePath
    $BundleFile = Split-Path -Leaf $BundlePath
    $Port = Get-FreePort
    $BundleUrl = "http://127.0.0.1:$Port/$BundleFile"

    $Server = Start-Process python -ArgumentList @(
        "-m", "http.server", $Port,
        "--bind", "127.0.0.1",
        "--directory", $BundleDirectory
    ) -PassThru -NoNewWindow

    $Ready = $false
    for ($Attempt = 0; $Attempt -lt 20; $Attempt++) {
        try {
            Invoke-WebRequest -Uri $BundleUrl -Method Head -UseBasicParsing | Out-Null
            $Ready = $true
            break
        }
        catch {
            Start-Sleep -Milliseconds 250
        }
    }
    if (-not $Ready) {
        throw "Bundle server did not become ready: $BundleUrl"
    }

    Invoke-Checked python @(
        "scripts/update_app_platform_urls.py",
        "--platform-url", $BundleUrl,
        "examples", "tests"
    )

    $Examples = Get-ChildItem examples -Filter *.roc | Sort-Object Name
    foreach ($Example in $Examples) {
        Invoke-Checked roc @("check", $Example.FullName, "--no-cache")
        Invoke-Checked roc @("test", $Example.FullName, "--no-cache")
        $Output = Join-Path $BuildDir ($Example.BaseName + ".exe")
        Invoke-Checked roc @("build", $Example.FullName, "--output=$Output", "--no-cache")
    }

    $Tests = Get-ChildItem tests -Filter *.roc |
        Where-Object { $_.Name -ne "cmd-test.roc" } |
        Sort-Object Name
    foreach ($Test in $Tests) {
        Invoke-Checked roc @("check", $Test.FullName, "--no-cache")
        $Output = Join-Path $BuildDir ($Test.BaseName + ".exe")
        Invoke-Checked roc @("build", $Test.FullName, "--output=$Output", "--no-cache")
    }

    Invoke-Checked -Command (Join-Path $BuildDir "hello.exe") -Arguments @()
}
finally {
    if ($Server -and -not $Server.HasExited) {
        Stop-Process -Id $Server.Id -Force
    }
    Copy-Item -Force (Join-Path $BackupDir "examples/*.roc") (Join-Path $RootDir "examples")
    Copy-Item -Force (Join-Path $BackupDir "tests/*.roc") (Join-Path $RootDir "tests")
    Pop-Location
    Remove-Item -Recurse -Force $TempDir
}
