use std::path::Path;

fn validate(path: &Path, png: &[u8]) -> Result<(), String> {
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
    {
        return Err("Choose a .png filename for the comparison proof.".into());
    }
    if png.len() < 24 || png.len() > 128 * 1024 * 1024 || !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Invalid or oversized comparison PNG.".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn save_comparison_proof(path: String, png: Vec<u8>) -> Result<(), String> {
    validate(Path::new(&path), &png)?;
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(path, png).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_accepts_png_data_at_png_paths() {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.resize(24, 0);
        assert!(validate(Path::new("proof.PNG"), &png).is_ok());
        assert!(validate(Path::new("video.mp4"), &png).is_err());
        assert!(validate(Path::new("proof.png"), b"not an image").is_err());
    }
}
