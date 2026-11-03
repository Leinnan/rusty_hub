param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Push-Location (Resolve-Path (Join-Path $PSScriptRoot '../../..'))
try {
    $rustHost = rustc -vV | Select-String '^host: x86_64-pc-windows-msvc$'
    if ($LASTEXITCODE -ne 0 -or !$rustHost) { throw 'Windows packaging requires an x64 MSVC Rust host.' }
    $wixPath = Join-Path (Get-Location) 'target/tools/wix/wix.exe'
    if (!(Test-Path -LiteralPath $wixPath)) {
        $wixCommand = Get-Command wix -ErrorAction SilentlyContinue
        if (!$wixCommand) { throw 'Run just windows-tools to install the pinned WiX toolchain.' }
        $wixPath = $wixCommand.Source
    }
    $wixVersion = & $wixPath --version
    if ($LASTEXITCODE -ne 0 -or $wixVersion -notlike '5.0.2*') { throw 'WiX 5.0.2 is required.' }
    if (!$SkipBuild) {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    }
    New-Item -ItemType Directory -Force -Path target/dist | Out-Null
    Copy-Item -LiteralPath target/release/rusty_hub.exe -Destination target/dist/rusty_hub.exe -Force
    cargo xtask icons
    if ($LASTEXITCODE -ne 0) { throw 'Icon generation failed.' }
    cargo xtask licenses
    if ($LASTEXITCODE -ne 0) { throw 'Licence list generation failed.' }
    $packageVersion = (Select-String -Path Cargo.toml -Pattern '^version = "([0-9]+\.[0-9]+\.[0-9]+)"$').Matches.Groups[1].Value
    if (!$packageVersion) { throw 'Expected a numeric workspace version for MSI.' }
    & $wixPath build -arch x64 -ext WixToolset.Util.wixext/5.0.2 -d "Version=$packageVersion" `
        assets/package/windows/Package.wxs `
        -o "target/dist/rusty-hub-$packageVersion-windows-x64.msi"
    if ($LASTEXITCODE -ne 0) { throw 'MSI build failed.' }
} finally { Pop-Location }
