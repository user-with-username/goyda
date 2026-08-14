use windows_sys::Win32::Graphics::Gdi::*;

use crate::components::Asset;

fn read_asset_file(path: &str) -> Option<Vec<u8>> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    std::fs::read(exe_dir.join("assets").join(path)).ok()
}

fn rasterize_svg(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let tree = resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).ok()?;
    let size = tree.size();
    let (width, height) = (size.width().ceil() as u32, size.height().ceil() as u32);
    if width == 0 || height == 0 {
        return None;
    }
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    Some((width, height, pixmap.data().to_vec()))
}

fn rgba_to_premultiplied_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len());
    for px in rgba.as_chunks::<4>().0 {
        let [r, g, b, a] = [px[0] as u32, px[1] as u32, px[2] as u32, px[3] as u32];
        out.push((b * a / 255) as u8);
        out.push((g * a / 255) as u8);
        out.push((r * a / 255) as u8);
        out.push(a as u8);
    }
    out
}

pub fn decode_to_bitmap(asset: &Asset) -> Option<(HBITMAP, i32, i32)> {
    let bytes = asset
        .bytes()
        .map(|b| b.to_vec())
        .or_else(|| read_asset_file(asset.path()))?;

    let (width, height, rgba) = if goyda_utils::asset::is_svg(asset) {
        rasterize_svg(&bytes)?
    } else {
        let decoded = image::load_from_memory(&bytes).ok()?.to_rgba8();
        let (w, h) = decoded.dimensions();
        (w, h, decoded.into_raw())
    };

    let bgra = rgba_to_premultiplied_bgra(&rgba);

    unsafe {
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width as i32;
        // Negative height marks a top-down DIB, matching the row order the
        // decoded image (and the SVG rasterizer) already produce.
        bmi.bmiHeader.biHeight = -(height as i32);
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let screen_dc = GetDC(std::ptr::null_mut());
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbitmap = CreateDIBSection(
            screen_dc,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        ReleaseDC(std::ptr::null_mut(), screen_dc);

        if hbitmap.is_null() || bits.is_null() {
            return None;
        }

        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());

        Some((hbitmap, width as i32, height as i32))
    }
}
