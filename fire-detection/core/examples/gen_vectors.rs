//! Regenerates `testdata/strip_vectors.txt` (shared with the Java mod tests).

use fire_core::sync::{stamp, strip_size};

fn main() {
    let mut out = String::new();
    for w in [854usize, 1280, 1920, 2560, 3840] {
        for (tick, id) in [
            (0x000000u32, 0x000000u32),
            (0x000001, 0xABCDEF),
            (0xFFFFFF, 0x000001),
            (0x123456, 0xFFFFFF),
        ] {
            let h = w * 9 / 16;
            let mut f = vec![0u8; w * h];
            stamp(&mut f, w, h, tick, id).unwrap();
            let (sw, _) = strip_size(w).unwrap();
            let row: String = f[..sw].iter().map(|&p| if p == 255 { '1' } else { '0' }).collect();
            out.push_str(&format!("{w} {tick:06X} {id:06X} {row}\n"));
        }
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/strip_vectors.txt");
    std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata")).unwrap();
    std::fs::write(path, out).unwrap();
    println!("wrote {path}");
}
