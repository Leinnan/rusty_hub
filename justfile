# Rusty Hub task runner (just 1.47+). `just` lists the recipes.

set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

version := if os_family() == "windows" {
    `((Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches.Groups[1].Value)`
} else {
    `sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1`
}
build := if os_family() == "windows" {
    `$count = git rev-list --count HEAD 2>$null; if ($LASTEXITCODE -eq 0) { $count } else { 0 }`
} else {
    `git rev-list --count HEAD 2>/dev/null || echo 0`
}
dist := "target/dist"
package := "target/package"
app := dist / "Rusty Hub.app"
app_id := "io.github.Leinnan.RustyHub"

default:
    @just --list

# Run the app (debug)
[group('dev')]
run:
    cargo run

# Run an optimized build
[group('dev')]
run-release:
    cargo run --release

# Run every workspace test
[group('code')]
test:
    cargo test --workspace

# Clippy with warnings as errors
[group('code')]
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Format all crates
[group('code')]
fmt:
    cargo fmt --all

# Format check, tests, clippy, and type checks for the other desktop targets
[group('code')]
validate:
    cargo fmt --all -- --check
    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo check --workspace --target x86_64-pc-windows-msvc
    cargo check --workspace --target x86_64-unknown-linux-gnu

# Re-record the egui_kittest snapshot images
[env('UPDATE_SNAPSHOTS', '1')]
[group('code')]
snapshots:
    cargo test --package rusty_hub

# Render assets/icon/icon.svg into target/package/icon for the bundles
[group('assets')]
icons:
    cargo xtask icons

# List the app's third-party crates in target/package/licenses.txt
[group('assets')]
licenses:
    cargo xtask licenses

# Build target/dist/Rusty Hub.app (release, ad-hoc signed)
[macos]
[group('package')]
app: icons licenses
    cargo build --release
    rm -rf "{{app}}"
    mkdir -p "{{app}}/Contents/MacOS" "{{app}}/Contents/Resources"
    cp target/release/rusty_hub "{{app}}/Contents/MacOS/rusty_hub"
    cp {{package}}/icon/rusty-hub.icns "{{app}}/Contents/Resources/rusty-hub.icns"
    cp LICENSE {{package}}/licenses.txt "{{app}}/Contents/Resources/"
    sed -e 's/@VERSION@/{{version}}/' -e 's/@BUILD@/{{build}}/' \
        assets/package/macos/Info.plist > "{{app}}/Contents/Info.plist"
    printf 'APPL????' > "{{app}}/Contents/PkgInfo"
    plutil -lint "{{app}}/Contents/Info.plist"
    codesign --force --sign - --timestamp=none "{{app}}"
    @echo "Built {{app}}"

# Build target/dist/rusty-hub-<version>.dmg with an Applications shortcut
[macos]
[group('package')]
dmg: app
    #!/bin/sh
    set -eu
    stage=$(mktemp -d)
    cp -R "{{app}}" "$stage/"
    ln -s /Applications "$stage/Applications"
    out="{{dist}}/rusty-hub-{{version}}.dmg"
    rm -f "$out"
    hdiutil create -quiet -volname "Rusty Hub {{version}}" -srcfolder "$stage" \
        -fs HFS+ -format UDZO -ov "$out"
    rm -rf "$stage"
    echo "Built $out"

# Copy the app into /Applications and register it
[macos]
[group('package')]
install: app
    rm -rf "/Applications/Rusty Hub.app"
    cp -R "{{app}}" /Applications/
    /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "/Applications/Rusty Hub.app"

# Release build with the icon and version info embedded (by build.rs)
[windows]
[group('package')]
exe:
    cargo build --release
    New-Item -ItemType Directory -Force -Path {{dist}} | Out-Null
    Copy-Item target/release/rusty_hub.exe {{dist}}/rusty_hub.exe

# Install the pinned, workspace-local WiX compiler and extension
[windows]
[group('package')]
windows-tools:
    dotnet tool install --tool-path target/tools/wix wix --version 5.0.2
    target/tools/wix/wix.exe extension add WixToolset.Util.wixext/5.0.2

# Build the unsigned per-user Windows 10/11 x64 installer
[windows]
[group('package')]
msi:
    powershell.exe -NoLogo -NoProfile -File assets/package/windows/package.ps1

# Install the MSI for the current user
[windows]
[group('package')]
exe-install: msi
    powershell.exe -NoLogo -NoProfile -File assets/package/windows/install.ps1

# Uninstall via Windows Installer (also available in Installed Apps)
[windows]
[group('package')]
exe-uninstall:
    powershell.exe -NoLogo -NoProfile -File assets/package/windows/install.ps1 -Uninstall

# Install the binary, .desktop entry and icons under PREFIX
[linux]
[group('package')]
linux-install prefix=(env_var('HOME') / ".local"): (_linux-tree prefix)
    -command -v update-desktop-database >/dev/null && update-desktop-database "{{prefix}}/share/applications"
    -command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "{{prefix}}/share/icons/hicolor"

# Build target/dist/rusty-hub-<version>-linux-<arch>.tar.gz with a usr/ tree
[linux]
[group('package')]
linux-package:
    rm -rf "{{dist}}/linux"
    just _linux-tree "{{dist}}/linux/usr"
    tar -czf "{{dist}}/rusty-hub-{{version}}-linux-$(uname -m).tar.gz" -C "{{dist}}/linux" usr
    @echo "Built {{dist}}/rusty-hub-{{version}}-linux-$(uname -m).tar.gz"

[linux]
_linux-tree prefix: icons licenses
    #!/bin/sh
    set -eu
    cargo build --release
    p="{{prefix}}"
    install -Dm755 target/release/rusty_hub "$p/bin/rusty_hub"
    install -Dm644 assets/package/linux/{{app_id}}.desktop "$p/share/applications/{{app_id}}.desktop"
    for size in 16 32 48 64 128 256 512; do
        install -Dm644 "{{package}}/icon/png/$size.png" "$p/share/icons/hicolor/${size}x${size}/apps/{{app_id}}.png"
    done
    install -Dm644 assets/icon/icon.svg "$p/share/icons/hicolor/scalable/apps/{{app_id}}.svg"
    install -Dm644 LICENSE "$p/share/doc/rusty-hub/LICENSE"
    install -Dm644 {{package}}/licenses.txt "$p/share/doc/rusty-hub/licenses.txt"

# Remove packaged outputs
[unix]
[group('package')]
clean-dist:
    rm -rf "{{dist}}" "{{package}}"

# Remove packaged outputs
[windows]
[group('package')]
clean-dist:
    if (Test-Path -LiteralPath '{{dist}}') { Remove-Item -LiteralPath '{{dist}}' -Recurse -Force -ErrorAction Stop }
    if (Test-Path -LiteralPath '{{package}}') { Remove-Item -LiteralPath '{{package}}' -Recurse -Force -ErrorAction Stop }
