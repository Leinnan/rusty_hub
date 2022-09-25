# Rusty Hub [![build](https://github.com/Leinnan/rusty_hub/actions/workflows/rust.yml/badge.svg)](https://github.com/Leinnan/rusty_hub/actions/workflows/rust.yml)

Very simple alternative for Unity Hub. Rust pet project.

![rusty_hub_egui_MfuZVmPa9T](https://user-images.githubusercontent.com/13188195/192160570-44f98c74-8193-4027-b0e4-5b7b0d02157c.gif)


## Building and using

[Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) is required in order to build it.

Building is pretty simple, just copy repo, open `egui_client` subdirectory in `CLI` client and run these commands:

```sh
git clone git@github.com:Leinnan/rusty_hub.git
cd rusty_hub/egui_client
cargo build --release
cargo run --release
```


## Thanks

Big thanks to https://github.com/unitycoder/UnityLauncherPro 

Most of the required information about how data is stored by Unity HUB is taken from there.
