use std::cell::RefCell;
use std::collections::HashMap;

use windows_sys::Win32::Graphics::Gdi::*;

thread_local! {
    static DEFAULT_FONTS: RefCell<HashMap<(i32, bool, bool, bool, bool), HFONT>> = RefCell::new(HashMap::new());
    static NAMED_FONTS: RefCell<HashMap<(String, i32), HFONT>> = RefCell::new(HashMap::new());
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn create_font(height: i32, face: &str, bold: bool, italic: bool, underline: bool, strikethrough: bool) -> HFONT {
    unsafe {
        CreateFontW(
            -height,
            0,
            0,
            0,
            if bold { FW_BOLD as i32 } else { FW_NORMAL as i32 },
            italic as u32,
            underline as u32,
            strikethrough as u32,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            (DEFAULT_PITCH | FF_DONTCARE) as u32,
            wide(face).as_ptr(),
        )
    }
}

/// The default UI font at a given pixel size (normal weight, upright),
/// cached per size. Equivalent to `styled_font(size_px, false, false, false, false)`.
pub fn default_font(size_px: i32) -> HFONT {
    styled_font(size_px, false, false, false, false)
}

/// The default UI font at a given pixel size/weight/slant/decoration,
/// cached per `(size, bold, italic, underline, strikethrough)` combination.
pub fn styled_font(size_px: i32, bold: bool, italic: bool, underline: bool, strikethrough: bool) -> HFONT {
    DEFAULT_FONTS.with(|cache| {
        *cache
            .borrow_mut()
            .entry((size_px, bold, italic, underline, strikethrough))
            .or_insert_with(|| create_font(size_px, "Segoe UI", bold, italic, underline, strikethrough))
    })
}

/// Registers `bytes` as a private, in-process font (via
/// `AddFontMemResourceEx`) and returns an `HFONT` at `size_px` using
/// whatever family name is embedded in the font's own `name` table -
/// callers never need to know or guess that name themselves, matching how
/// `Typeface.Builder(byte[])` (Android) and a `Blob` `@font-face` (web) both
/// build a usable font directly from bytes with no separate lookup step.
pub fn font_from_bytes(bytes: &[u8], size_px: i32) -> Option<HFONT> {
    let family = parse_family_name(bytes)?;

    if let Some(font) = NAMED_FONTS.with(|c| c.borrow().get(&(family.clone(), size_px)).copied()) {
        return Some(font);
    }

    unsafe {
        let mut installed = 0u32;
        let handle = AddFontMemResourceEx(bytes.as_ptr() as *const _, bytes.len() as u32, std::ptr::null(), &mut installed);
        if handle.is_null() {
            return None;
        }
    }

    let font = create_font(size_px, &family, false, false, false, false);
    NAMED_FONTS.with(|c| c.borrow_mut().insert((family, size_px), font));
    Some(font)
}

/// Minimal TTF/OTF `name` table parser - just enough to pull out the
/// Windows-platform Family Name (`nameID` 1) record, so an embedded font's
/// bytes are self-describing and don't need a name supplied alongside them.
fn parse_family_name(data: &[u8]) -> Option<String> {
    let u16_at = |o: usize| -> Option<u16> { Some(u16::from_be_bytes([*data.get(o)?, *data.get(o + 1)?])) };
    let u32_at = |o: usize| -> Option<u32> {
        Some(u32::from_be_bytes([*data.get(o)?, *data.get(o + 1)?, *data.get(o + 2)?, *data.get(o + 3)?]))
    };

    let num_tables = u16_at(4)? as usize;
    let mut name_table_offset = None;
    for i in 0..num_tables {
        let entry = 12 + i * 16;
        let tag = data.get(entry..entry + 4)?;
        if tag == b"name" {
            name_table_offset = Some(u32_at(entry + 8)? as usize);
            break;
        }
    }
    let table = name_table_offset?;

    let count = u16_at(table + 2)? as usize;
    let string_offset = u16_at(table + 4)? as usize;

    for i in 0..count {
        let record = table + 6 + i * 12;
        let platform_id = u16_at(record)?;
        let encoding_id = u16_at(record + 2)?;
        let name_id = u16_at(record + 6)?;
        let length = u16_at(record + 8)? as usize;
        let offset = u16_at(record + 10)? as usize;

        if name_id != 1 || platform_id != 3 || encoding_id != 1 {
            continue;
        }

        let start = table + string_offset + offset;
        let raw = data.get(start..start + length)?;
        let units: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        if let Ok(name) = String::from_utf16(&units) {
            return Some(name);
        }
    }

    None
}
