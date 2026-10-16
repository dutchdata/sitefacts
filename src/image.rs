//! Width and height of a PNG or WebP from its header, so facts and files cannot disagree.

use std::path::Path;

pub fn size(path: &Path) -> Result<(u32, u32), String> {
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    dims(&data).ok_or_else(|| format!("{}: not a PNG or WebP", path.display()))
}

pub fn dims(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() >= 24 && data[..8] == [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'] {
        return Some((u32::from_be_bytes(data[16..20].try_into().ok()?), u32::from_be_bytes(data[20..24].try_into().ok()?)));
    }
    if data.len() >= 30 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return match &data[12..16] {
            b"VP8X" => Some((le24(&data[24..27]) + 1, le24(&data[27..30]) + 1)),
            b"VP8L" => { let bits = u32::from_le_bytes(data[21..25].try_into().ok()?); Some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1)) }
            b"VP8 " => Some(((u16::from_le_bytes([data[26], data[27]]) & 0x3FFF) as u32, (u16::from_le_bytes([data[28], data[29]]) & 0x3FFF) as u32)),
            _ => None,
        };
    }
    None
}
fn le24(b: &[u8]) -> u32 { b[0] as u32 | (b[1] as u32) << 8 | (b[2] as u32) << 16 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn png_header() {
        let mut d = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n', 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
        d.extend_from_slice(&1200u32.to_be_bytes());
        d.extend_from_slice(&630u32.to_be_bytes());
        assert_eq!(dims(&d), Some((1200, 630)));
        assert_eq!(dims(b"nope"), None);
    }
}
