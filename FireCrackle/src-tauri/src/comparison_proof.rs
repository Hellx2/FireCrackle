use tauri_plugin_clipboard_manager::ClipboardExt;

fn decode(png: &[u8]) -> Result<tauri::image::Image<'static>, String> {
    if png.len() < 24 || png.len() > 128 * 1024 * 1024 || !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Invalid or oversized comparison PNG.".into());
    }
    let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
    if width == 0
        || height == 0
        || width > 16384
        || height > 16384
        || u64::from(width) * u64::from(height) > 64_000_000
    {
        return Err("Invalid or oversized comparison dimensions.".into());
    }
    tauri::image::Image::from_bytes(png).map_err(|e| format!("Could not read comparison PNG: {e}"))
}

#[tauri::command]
pub async fn copy_comparison_proof(app: tauri::AppHandle, png: Vec<u8>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let image = decode(&png)?;
        app.clipboard()
            .write_image(&image)
            .map_err(|e| format!("Could not copy comparison image: {e}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_png_to_clipboard_pixels_and_rejects_invalid_data() {
        let png = include_bytes!("../icons/32x32.png");
        let image = decode(png).unwrap();
        assert_eq!((image.width(), image.height()), (32, 32));
        assert_eq!(image.rgba().len(), 32 * 32 * 4);
        assert!(decode(b"not an image").is_err());
        let mut oversized = png.to_vec();
        oversized[16..20].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(decode(&oversized).is_err());
    }
}
