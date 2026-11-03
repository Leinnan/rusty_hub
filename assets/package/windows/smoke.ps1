# Run only on a disposable Windows CI runner or clean test VM.
# This script installs, repairs, upgrades and uninstalls Rusty Hub.
param([Parameter(Mandatory)][string]$MsiPath)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$MsiPath = (Resolve-Path -LiteralPath $MsiPath).Path
$testOutput = Join-Path (Get-Location) 'target/installer-smoke'
New-Item -ItemType Directory -Force -Path $testOutput | Out-Null
$installDir = Join-Path $env:LOCALAPPDATA 'Programs/rusty_hub'
$shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Rusty Hub/Rusty Hub.lnk'
$testProcess = $null
$activeMsi = $MsiPath

function Invoke-TestMsi([string]$Operation, [string]$Package, [string]$Label, [int[]]$Expected = @(0, 3010)) {
    $log = Join-Path $testOutput "$Label.log"
    $process = Start-Process -WindowStyle Hidden -Wait -PassThru msiexec.exe `
        -ArgumentList @($Operation, "`"$Package`"", '/qn', '/norestart', '/L*v', "`"$log`"")
    if ($process.ExitCode -notin $Expected) { throw "$Label returned $($process.ExitCode); see $log" }
}

try {
    Invoke-TestMsi '/i' $MsiPath 'install'
    if (!(Test-Path -LiteralPath (Join-Path $installDir 'rusty_hub.exe'))) { throw 'Executable missing after install.' }
    if (!(Test-Path -LiteralPath $shortcut)) { throw 'Start menu shortcut missing.' }
    # An unowned file must survive upgrade and uninstall.
    $sentinel = Join-Path $installDir 'user-content.txt'
    Set-Content -LiteralPath $sentinel -Value 'Keep this user content.'
    Invoke-TestMsi '/fa' $MsiPath 'repair'

    $testProcess = Start-Process -WindowStyle Hidden -PassThru (Join-Path $installDir 'rusty_hub.exe')
    Start-Sleep -Seconds 2
    $testProcess.Refresh()
    if ($testProcess.HasExited) { throw 'Installed executable failed to start.' }
    Invoke-TestMsi '/fa' $MsiPath 'running-app-blocked' @(1603)
    $testProcess.Refresh()
    if ($testProcess.HasExited) { throw 'Installer closed the running application.' }
    if (!$testProcess.CloseMainWindow() -or !$testProcess.WaitForExit(10000)) {
        # This fresh test instance has nothing to save.
        Stop-Process -Id $testProcess.Id -ErrorAction SilentlyContinue
    }
    $testProcess = $null

    $baseVersion = (Select-String -Path Cargo.toml -Pattern '^version = "([0-9]+\.[0-9]+\.[0-9]+)"$').Matches.Groups[1].Value
    $parts = $baseVersion.Split('.')
    $upgradeVersion = "$($parts[0]).$($parts[1]).$([int]$parts[2] + 1)"
    $upgradeMsi = Join-Path $testOutput 'upgrade.msi'
    $wixPath = Join-Path (Get-Location) 'target/tools/wix/wix.exe'
    $rebuiltMsi = Join-Path $testOutput 'same-version.msi'
    & $wixPath build -arch x64 -ext WixToolset.Util.wixext/5.0.2 -d "Version=$baseVersion" `
        assets/package/windows/Package.wxs -o $rebuiltMsi
    if ($LASTEXITCODE -ne 0) { throw 'Same-version fixture build failed.' }
    Invoke-TestMsi '/i' $rebuiltMsi 'same-version-upgrade'
    $activeMsi = $rebuiltMsi
    & $wixPath build -arch x64 -ext WixToolset.Util.wixext/5.0.2 -d "Version=$upgradeVersion" `
        assets/package/windows/Package.wxs -o $upgradeMsi
    if ($LASTEXITCODE -ne 0) { throw 'Upgrade fixture build failed.' }
    Invoke-TestMsi '/i' $upgradeMsi 'upgrade'
    $activeMsi = $upgradeMsi
    Invoke-TestMsi '/i' $MsiPath 'downgrade-blocked' @(1603, 1638)
    Invoke-TestMsi '/x' $activeMsi 'uninstall'
    $activeMsi = $null
    if (Test-Path -LiteralPath (Join-Path $installDir 'rusty_hub.exe')) { throw 'Executable survived uninstall.' }
    if (Test-Path -LiteralPath $shortcut) { throw 'Shortcut survived uninstall.' }
    if (!(Test-Path -LiteralPath $sentinel)) { throw 'Uninstall deleted user content.' }
    if (Test-Path -LiteralPath 'HKCU:/Software/rusty_hub.Installed') {
        $registered = Get-ItemProperty -LiteralPath 'HKCU:/Software/rusty_hub.Installed'
        if ($registered.PSObject.Properties.Name -contains 'BinaryInstalled') { throw 'Registration survived uninstall.' }
    }
    Remove-Item -LiteralPath $sentinel
    Write-Output 'Installer lifecycle checks passed.'
} finally {
    if ($null -ne $testProcess) { Stop-Process -Id $testProcess.Id -ErrorAction SilentlyContinue }
    if ($activeMsi) { Invoke-TestMsi '/x' $activeMsi 'cleanup' @(0, 1605, 3010) }
}
