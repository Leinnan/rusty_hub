param([switch]$Uninstall)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Push-Location (Resolve-Path (Join-Path $PSScriptRoot '../../..'))
try {
    $packageVersion = (Select-String -Path Cargo.toml -Pattern '^version = "([0-9]+\.[0-9]+\.[0-9]+)"$').Matches.Groups[1].Value
    $msiPath = (Resolve-Path "target/dist/rusty-hub-$packageVersion-windows-x64.msi").Path
    $operation = if ($Uninstall) { '/x' } else { '/i' }
    $process = Start-Process -WindowStyle Hidden -Wait -PassThru -FilePath msiexec.exe `
        -ArgumentList @($operation, "`"$msiPath`"", '/passive', '/norestart')
    if ($process.ExitCode -notin @(0, 3010)) { throw "Windows Installer failed: $($process.ExitCode)" }
} finally { Pop-Location }
