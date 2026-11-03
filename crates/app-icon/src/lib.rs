//! Renders the app icon from `assets/icon/icon.svg`. The SVG is the only source:
//! nothing rendered from it is committed.

use std::error::Error;
use std::io::Cursor;
use std::path::Path;

use resvg::{
    tiny_skia::{Pixmap, Transform},
    usvg,
};

pub use resvg::usvg::Tree;

/// Size of the window icon (px).
pub const WINDOW_SIZE: u32 = 512;
/// Sizes inside the Windows `.ico` (px).
pub const ICO_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];
/// Largest size Windows tools reliably take as BMP data in an `.ico` (px).
const ICO_BMP_BELOW: u32 = 256;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Read the SVG at `path`.
pub fn load(path: &Path) -> Result<Tree> {
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(usvg::Tree::from_data(&data, &usvg::Options::default())?)
}

fn render(tree: &Tree, size: u32) -> Result<Pixmap> {
    let mut pixmap = Pixmap::new(size, size).ok_or("the icon size is zero")?;
    let scale = size as f32 / tree.size().width();
    resvg::render(
        tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

/// The icon as PNG data.
pub fn render_png(tree: &Tree, size: u32) -> Result<Vec<u8>> {
    Ok(render(tree, size)?.encode_png()?)
}

/// The icon as plain (not premultiplied) RGBA pixels.
pub fn render_rgba(tree: &Tree, size: u32) -> Result<Vec<u8>> {
    Ok(render(tree, size)?
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect())
}

/// The icon as `.ico` data.
pub fn render_ico(tree: &Tree) -> Result<Vec<u8>> {
    let mut icons = ico::IconDir::new(ico::ResourceType::Icon);
    for size in ICO_SIZES {
        let image = ico::IconImage::read_png(Cursor::new(render_png(tree, size)?))?;
        // Windows tools only reliably take PNG data in the largest entry.
        icons.add_entry(if size < ICO_BMP_BELOW {
            ico::IconDirEntry::encode_as_bmp(&image)?
        } else {
            ico::IconDirEntry::encode_as_png(&image)?
        });
    }
    let mut data = Vec::new();
    icons.write(&mut data)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::Path;

    use super::{ICO_SIZES, Tree, load, render_ico, render_png, render_rgba};

    fn tree() -> Tree {
        let svg = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icon/icon.svg");
        load(&svg).expect("the icon source parses")
    }

    #[test]
    fn pngs_decode_at_their_size() {
        for size in [16, 512] {
            let png = render_png(&tree(), size).expect("rendered");
            let image = ico::IconImage::read_png(Cursor::new(png)).expect("a valid PNG");
            assert_eq!((image.width(), image.height()), (size, size));
        }
    }

    #[test]
    fn the_ico_has_every_size_and_png_only_at_the_top() {
        let data = render_ico(&tree()).expect("rendered");
        let icons = ico::IconDir::read(Cursor::new(data)).expect("a valid ICO");
        let sizes: Vec<u32> = icons
            .entries()
            .iter()
            .map(ico::IconDirEntry::width)
            .collect();
        assert_eq!(sizes, ICO_SIZES);
        for entry in icons.entries() {
            assert_eq!(entry.is_png(), entry.width() == 256);
        }
    }

    #[test]
    fn pixels_are_rgba_with_transparent_corners() {
        let pixels = render_rgba(&tree(), 64).expect("rendered");
        assert_eq!(pixels.len(), 64 * 64 * 4);
        assert_eq!(pixels[3], 0, "the plate has rounded corners");
    }

    #[test]
    fn a_missing_source_says_where() {
        let error = load(Path::new("/nonexistent/icon.svg")).expect_err("no file");
        assert!(error.to_string().contains("/nonexistent/icon.svg"));
    }
}
