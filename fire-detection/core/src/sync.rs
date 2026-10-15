/// Number of cells in the strip: 2 calibration + 24 tick + 24 frame id + 8 crc.
pub const CELLS: usize = 58;
/// Largest encodable tick (24 bits).
pub const MAX_TICK: u32 = 0xFF_FFFF;
/// Largest encodable frame id (24 bits).
pub const MAX_ID: u32 = 0xFF_FFFF;

const MIN_CELL: f64 = 2.0;
const MIN_GAP: f32 = 60.0;

/// What the strip carries: the game tick and the mod's render-frame id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncCode {
    pub tick: u32,
    pub frame_id: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SyncError {
    TickOutOfRange,
    IdOutOfRange,
    FrameTooSmall,
    BadBufferLength,
}

/// CRC-8, poly 0x07, init 0, no reflection, no xorout.
pub fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Cell size in pixels for a video of this width (fractional, 8 px at 1920 wide).
pub fn cell_size(width: usize) -> f64 {
    8.0 * width as f64 / 1920.0
}

/// Strip size `(w, h)` in pixels for a video of this width, or `None` if the cells
/// would be smaller than 2 px.
pub fn strip_size(width: usize) -> Option<(usize, usize)> {
    let c = cell_size(width);
    if c < MIN_CELL {
        return None;
    }
    Some(((CELLS as f64 * c).round() as usize, c.round() as usize))
}

/// CRC over the 6 payload bytes: tick (3, big-endian) then frame id (3, big-endian).
fn code_crc(tick: u32, frame_id: u32) -> u8 {
    crc8(&[
        (tick >> 16) as u8,
        (tick >> 8) as u8,
        tick as u8,
        (frame_id >> 16) as u8,
        (frame_id >> 8) as u8,
        frame_id as u8,
    ])
}

/// Draw the strip for `tick` and `frame_id` into the top-left of an 8-bit luma frame.
pub fn stamp(
    luma: &mut [u8],
    width: usize,
    height: usize,
    tick: u32,
    frame_id: u32,
) -> Result<(), SyncError> {
    if tick > MAX_TICK {
        return Err(SyncError::TickOutOfRange);
    }
    if frame_id > MAX_ID {
        return Err(SyncError::IdOutOfRange);
    }
    if luma.len() != width * height {
        return Err(SyncError::BadBufferLength);
    }
    let (_, strip_h) = strip_size(width).ok_or(SyncError::FrameTooSmall)?;
    if height < strip_h {
        return Err(SyncError::FrameTooSmall);
    }

    // 56 bits, MSB first: tick (24), frame id (24), crc (8)
    let word = ((tick as u64) << 32) | ((frame_id as u64) << 8) | code_crc(tick, frame_id) as u64;
    let c = cell_size(width);
    for i in 0..CELLS {
        let white = match i {
            0 => true,
            1 => false,
            _ => (word >> (55 - (i - 2))) & 1 == 1,
        };
        let value = if white { 255 } else { 0 };
        let x0 = (i as f64 * c).round() as usize;
        let x1 = ((i + 1) as f64 * c).round() as usize;
        for y in 0..strip_h {
            luma[y * width + x0..y * width + x1].fill(value);
        }
    }
    Ok(())
}

/// Mean luma of the central half of strip cell `i`.
fn cell_mean(luma: &[u8], width: usize, c: f64, i: usize) -> f32 {
    // Pixels whose centers lie in the central half of the cell.
    let span = |start: f64| {
        let lo = (start + c / 4.0 - 0.5).ceil().max(0.0) as usize;
        let hi = (start + 3.0 * c / 4.0 - 0.5).ceil().max(0.0) as usize;
        (lo, hi.max(lo + 1))
    };
    let (x_lo, x_hi) = span(i as f64 * c);
    let (y_lo, y_hi) = span(0.0);
    let mut sum = 0u32;
    for y in y_lo..y_hi {
        let row = &luma[y * width + x_lo..y * width + x_hi];
        sum += row.iter().map(|&p| p as u32).sum::<u32>();
    }
    sum as f32 / ((x_hi - x_lo) * (y_hi - y_lo)) as f32
}

/// Read the tick and frame id back from an 8-bit luma frame. `None` if there is no valid strip.
pub fn decode(luma: &[u8], width: usize, height: usize) -> Option<SyncCode> {
    if luma.len() != width * height {
        return None;
    }
    let (_, strip_h) = strip_size(width)?;
    if height < strip_h {
        return None;
    }
    let c = cell_size(width);

    let white = cell_mean(luma, width, c, 0);
    let black = cell_mean(luma, width, c, 1);
    let gap = white - black;
    if gap < MIN_GAP {
        return None;
    }
    let threshold = (white + black) / 2.0;
    let dead_zone = gap / 8.0;

    let mut word = 0u64;
    for i in 2..CELLS {
        let m = cell_mean(luma, width, c, i);
        if (m - threshold).abs() < dead_zone {
            return None;
        }
        word = (word << 1) | (m > threshold) as u64;
    }
    let tick = ((word >> 32) & 0xFF_FFFF) as u32;
    let frame_id = ((word >> 8) & 0xFF_FFFF) as u32;
    (code_crc(tick, frame_id) == (word & 0xFF) as u8).then_some(SyncCode { tick, frame_id })
}
