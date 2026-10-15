//! `testdata/strip_vectors.txt` is shared with the Java mod's tests. Each line:
//! `<video width> <tick, 6 hex digits> <frame id, 6 hex digits> <first strip row, one char per pixel: 1 = white, 0 = black>`.
//! If this test fails, the format changed: regenerate with
//! `cargo run --example gen_vectors` AND make sure the Java side still passes.

use fire_core::sync::{stamp, strip_size};

const WIDTHS: [usize; 5] = [854, 1280, 1920, 2560, 3840];
const PAIRS: [(u32, u32); 4] = [
    (0x000000, 0x000000),
    (0x000001, 0xABCDEF),
    (0xFFFFFF, 0x000001),
    (0x123456, 0xFFFFFF),
];

fn expected_line(width: usize, tick: u32, id: u32) -> String {
    let height = width * 9 / 16;
    let mut f = vec![0u8; width * height];
    stamp(&mut f, width, height, tick, id).unwrap();
    let (sw, _) = strip_size(width).unwrap();
    let row: String = f[..sw].iter().map(|&p| if p == 255 { '1' } else { '0' }).collect();
    format!("{width} {tick:06X} {id:06X} {row}")
}

#[test]
fn golden_file_matches_the_codec() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/strip_vectors.txt");
    let text = std::fs::read_to_string(path).expect("testdata/strip_vectors.txt missing");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), WIDTHS.len() * PAIRS.len());

    let mut i = 0;
    for w in WIDTHS {
        for (tick, id) in PAIRS {
            assert_eq!(lines[i], expected_line(w, tick, id), "line {i}");
            i += 1;
        }
    }
}

#[test]
fn golden_rows_are_internally_sane() {
    // Calibration cells come first: white then black, whatever the width.
    for w in WIDTHS {
        let line = expected_line(w, 0x123456, 0x654321);
        let row = line.split(' ').nth(3).unwrap();
        let c = 8.0 * w as f64 / 1920.0;
        assert_eq!(row.as_bytes()[(c * 0.5) as usize], b'1', "{w}: cell 0 white");
        assert_eq!(row.as_bytes()[(c * 1.5) as usize], b'0', "{w}: cell 1 black");
    }
}
