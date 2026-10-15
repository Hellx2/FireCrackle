//! The mod's label log (JSONL): finding a row by frame id without loading the file into memory.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

/// Reads `(frame id, tick)` from the start of a row. Strict on purpose: it only accepts the exact start the mod
/// writes, `{"f":<id>,"tick":<tick>,`, so a row in any other shape is reported as bad instead of misread.
pub fn parse_label_head(line: &str) -> Option<(u32, u64)> {
    let rest = line.strip_prefix("{\"f\":")?;
    let end = rest.find(|c: char| !c.is_ascii_digit())?;
    if end == 0 {
        return None;
    }
    let id: u32 = rest[..end].parse().ok()?;
    let rest = rest[end..].strip_prefix(",\"tick\":")?;
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let tick: u64 = rest[..end].parse().ok()?;
    Some((id, tick))
}

/// Maps frame id -> (tick, byte offset of its row). Rows are read back from the file on demand.
pub struct LabelIndex {
    reader: BufReader<File>,
    pos: u64,
    map: HashMap<u32, (u64, u64)>,
    /// Valid rows seen (duplicates included).
    pub rows: usize,
    /// Non-blank lines that were not a valid row.
    pub bad_lines: usize,
    /// Rows whose id had already been seen (24-bit wrap or a concatenated log); the first one wins.
    pub duplicate_ids: usize,
}

fn trim_eol(buf: &mut Vec<u8>) {
    while matches!(buf.last(), Some(b'\n') | Some(b'\r')) {
        buf.pop();
    }
}

impl LabelIndex {
    pub fn build(path: &Path) -> io::Result<LabelIndex> {
        let mut reader = BufReader::new(File::open(path)?);
        let mut map = HashMap::new();
        let (mut rows, mut bad_lines, mut duplicate_ids) = (0usize, 0usize, 0usize);
        let mut offset = 0u64;
        let mut buf = Vec::new();
        loop {
            buf.clear();
            let n = reader.read_until(b'\n', &mut buf)?;
            if n == 0 {
                break;
            }
            let line_offset = offset;
            offset += n as u64;
            trim_eol(&mut buf);
            if buf.is_empty() {
                continue;
            }
            match parse_label_head(&String::from_utf8_lossy(&buf)) {
                Some((id, tick)) => {
                    rows += 1;
                    if map.contains_key(&id) {
                        duplicate_ids += 1;
                    } else {
                        map.insert(id, (tick, line_offset));
                    }
                }
                None => bad_lines += 1,
            }
        }
        Ok(LabelIndex { reader, pos: offset, map, rows, bad_lines, duplicate_ids })
    }

    /// The tick and the raw text of the row for `id`, if there is one.
    pub fn get(&mut self, id: u32) -> io::Result<Option<(u64, String)>> {
        let Some(&(tick, offset)) = self.map.get(&id) else { return Ok(None) };
        self.reader.seek_relative(offset as i64 - self.pos as i64)?;
        let mut buf = Vec::new();
        let n = self.reader.read_until(b'\n', &mut buf)?;
        self.pos = offset + n as u64;
        trim_eol(&mut buf);
        Ok(Some((tick, String::from_utf8_lossy(&buf).into_owned())))
    }
}
