//! Reading video frames from a pipe, and understanding `ffprobe` output.

use std::io::{self, ErrorKind, Read};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct VideoInfo {
    pub width: usize,
    pub height: usize,
    pub fps_num: u32,
    pub fps_den: u32,
}

fn parse_dim(key: &str, v: &str) -> Result<usize, String> {
    match v.trim().parse::<usize>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(format!("bad {key} in ffprobe output: {v:?}")),
    }
}

/// Parses `key=value` lines as printed by `ffprobe -of default=noprint_wrappers=1`. Order does not matter and
/// unknown keys are ignored; if several streams are printed the first value of each key wins.
pub fn parse_probe(text: &str) -> Result<VideoInfo, String> {
    let mut width = None;
    let mut height = None;
    let mut fps = None;
    for line in text.lines() {
        let Some((key, value)) = line.trim().split_once('=') else { continue };
        match key {
            "width" if width.is_none() => width = Some(parse_dim("width", value)?),
            "height" if height.is_none() => height = Some(parse_dim("height", value)?),
            "r_frame_rate" if fps.is_none() => {
                let (n, d) = value
                    .trim()
                    .split_once('/')
                    .ok_or_else(|| format!("bad frame rate in ffprobe output: {value:?}"))?;
                let n: u32 = n.parse().map_err(|_| format!("bad frame rate: {value:?}"))?;
                let d: u32 = d.parse().map_err(|_| format!("bad frame rate: {value:?}"))?;
                if n == 0 || d == 0 {
                    return Err(format!("unusable frame rate: {value:?}"));
                }
                fps = Some((n, d));
            }
            _ => {}
        }
    }
    let width = width.ok_or("ffprobe output has no width")?;
    let height = height.ok_or("ffprobe output has no height")?;
    let (fps_num, fps_den) = fps.ok_or("ffprobe output has no frame rate")?;
    Ok(VideoInfo { width, height, fps_num, fps_den })
}

/// Fills `buf` completely from `r`. `Ok(false)` is a clean end of stream (nothing read at all); a stream that ends
/// in the middle of a frame is an `UnexpectedEof` error. Short reads from a pipe are stitched together.
pub fn read_frame<R: Read>(r: &mut R, buf: &mut [u8]) -> io::Result<bool> {
    if buf.is_empty() {
        return Err(io::Error::new(ErrorKind::InvalidInput, "frame buffer is empty"));
    }
    let mut filled = 0;
    while filled < buf.len() {
        match r.read(&mut buf[filled..]) {
            Ok(0) => {
                return if filled == 0 {
                    Ok(false)
                } else {
                    Err(io::Error::new(ErrorKind::UnexpectedEof, format!("stream ended {filled} bytes into a {} byte frame", buf.len())))
                };
            }
            Ok(n) => filled += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(true)
}
