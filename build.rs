//! Renders the app icon from its SVG source: a PNG for the window, and on
//! Windows an ICO embedded in the executable with its version info.

use std::{env, error::Error, fs, path::Path};

const SOURCE: &str = "assets/icon/icon.svg";

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed={SOURCE}");
    println!("cargo:rerun-if-changed=assets/package/windows/app.manifest");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR")?;
    let out_dir = Path::new(&out_dir);
    let tree = app_icon::load(Path::new(SOURCE))?;

    fs::write(
        out_dir.join("icon.png"),
        app_icon::render_png(&tree, app_icon::WINDOW_SIZE)?,
    )?;
    let ico_path = out_dir.join("icon.ico");
    fs::write(&ico_path, app_icon::render_ico(&tree)?)?;

    // Cross-compiling would need a resource compiler for the target. Releases
    // for Windows are built on Windows.
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon(ico_path.to_str().ok_or("OUT_DIR isn't valid UTF-8")?)
            .set_manifest_file("assets/package/windows/app.manifest")
            .set("ProductName", "Rusty Hub")
            .set("FileDescription", "Rusty Hub")
            .set("LegalCopyright", "Copyright (c) 2022 Piotr Siuszko");
        resource.compile()?;
    }
    Ok(())
}
