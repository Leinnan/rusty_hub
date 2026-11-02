//! Renders the app icon from its SVG source: a PNG for the window, and an ICO
//! that is embedded in the executable on Windows.

use std::{env, error::Error, fs, io::Cursor, path::Path};

use resvg::{
    tiny_skia::{Pixmap, Transform},
    usvg,
};

const SOURCE: &str = "assets/icon/icon.svg";
/// Size of the window icon (px).
const WINDOW_ICON_SIZE: u32 = 512;
/// Sizes inside the ICO (px).
const ICO_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed={SOURCE}");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR")?;
    let out_dir = Path::new(&out_dir);
    let tree = usvg::Tree::from_data(&fs::read(SOURCE)?, &usvg::Options::default())?;

    fs::write(
        out_dir.join("icon.png"),
        render_png(&tree, WINDOW_ICON_SIZE)?,
    )?;

    let ico_path = out_dir.join("icon.ico");
    let mut icons = ico::IconDir::new(ico::ResourceType::Icon);
    for size in ICO_SIZES {
        let image = ico::IconImage::read_png(Cursor::new(render_png(&tree, size)?))?;
        // Windows tools only reliably take PNG data in the largest entry.
        icons.add_entry(if size < 256 {
            ico::IconDirEntry::encode_as_bmp(&image)?
        } else {
            ico::IconDirEntry::encode_as_png(&image)?
        });
    }
    icons.write(fs::File::create(&ico_path)?)?;

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(ico_path.to_str().ok_or("OUT_DIR isn't valid UTF-8")?);
        resource.compile()?;
    }
    Ok(())
}

fn render_png(tree: &usvg::Tree, size: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut pixmap = Pixmap::new(size, size).ok_or("the icon size is zero")?;
    let scale = size as f32 / tree.size().width();
    resvg::render(
        tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap.encode_png()?)
}
