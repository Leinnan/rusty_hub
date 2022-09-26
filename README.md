# Rusty Hub [![build](https://github.com/Leinnan/rusty_hub/actions/workflows/rust.yml/badge.svg)](https://github.com/Leinnan/rusty_hub/actions/workflows/rust.yml)

Very simple alternative for Unity Hub. Rust pet project. For now it does work properly only on Windows, but support for other platforms is on the roadmap.

![rusty_hub_egui_JU3JdNtfpz](https://user-images.githubusercontent.com/13188195/192162924-2f8eaef5-fc65-47f2-834c-f8abb704451d.gif)

## Download

There is ready to download Windows version. Go to [Releases](https://github.com/Leinnan/rusty_hub/releases) and look for `windows-build.zip` in `Assets`.

## Building and using

[Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) is required in order to build it.

Building is pretty simple, just copy repo, open `rusty_hub_egui` subdirectory in `CLI` client and run these commands:

```sh
git clone git@github.com:Leinnan/rusty_hub.git
cd rusty_hub/rusty_hub_egui
cargo build --release
cargo run --release
```


## Thanks

Big thanks to https://github.com/unitycoder/UnityLauncherPro 

Most of the required information about how data is stored by Unity HUB is taken from there.

Thanks for the icon to the [Papirus Development Team](https://github.com/PapirusDevelopmentTeam/papirus-icon-theme/)
