//! RGB 画像バッファと決定的 PNG エンコーダ。
//!
//! 外部依存ゼロ。deflate "store" ブロック (非圧縮) で valid PNG を生成する。
//! ファイルサイズは大きいが ≤2秒の screenshot KPI は達成できる (問7)。
//! 決定性: 同一画素列 → バイト同一 PNG (問5)。

/// RGB 画像バッファ。原点は左上、row-major。
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>, // R,G,B 各 u8、stride = width*3
}

impl Image {
    pub fn new(width: usize, height: usize, bg: [u8; 3]) -> Image {
        let mut pixels = vec![0u8; width * height * 3];
        for chunk in pixels.chunks_exact_mut(3) {
            chunk.copy_from_slice(&bg);
        }
        Image {
            width,
            height,
            pixels,
        }
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, rgb: [u8; 3]) {
        let off = (y * self.width + x) * 3;
        self.pixels[off..off + 3].copy_from_slice(&rgb);
    }

    /// 決定的 PNG バイト列 (deflate store; 外部依存ゼロ)。
    pub fn encode_png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        // PNG signature
        out.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

        // IHDR
        let mut ihdr = [0u8; 13];
        ihdr[0..4].copy_from_slice(&(self.width as u32).to_be_bytes());
        ihdr[4..8].copy_from_slice(&(self.height as u32).to_be_bytes());
        ihdr[8] = 8; // bit depth
        ihdr[9] = 2; // color type: RGB
                     // compression/filter/interlace = 0
        write_chunk(&mut out, b"IHDR", &ihdr);

        // 各スキャンラインにフィルタバイト 0x00 を付加
        let stride = self.width * 3;
        let raw_len = self.height * (1 + stride);
        let mut raw = Vec::with_capacity(raw_len);
        for row in 0..self.height {
            raw.push(0x00); // no filter
            raw.extend_from_slice(&self.pixels[row * stride..(row + 1) * stride]);
        }

        // IDAT = zlib(deflate store blocks)
        let idat = zlib_store(&raw);
        write_chunk(&mut out, b"IDAT", &idat);

        // IEND
        write_chunk(&mut out, b"IEND", &[]);
        out
    }

    pub fn write_png(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.encode_png())
    }
}

// ── PNG helpers ───────────────────────────────────────────────────────────────

fn write_chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(tag);
    out.extend_from_slice(data);
    let crc = crc32(tag, data);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// zlib wrapper around deflate "stored" (type-0) blocks。
fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    // zlib header: CMF=0x78 (deflate, window=32KB), FLG makes it divisible by 31
    out.push(0x78);
    out.push(0x01);

    // deflate stored blocks (max 65535 bytes each)
    let mut pos = 0;
    while pos < data.len() {
        let end = (pos + 65535).min(data.len());
        let bfinal = if end == data.len() { 1u8 } else { 0u8 };
        let len = (end - pos) as u16;
        out.push(bfinal); // BFINAL=1 if last, BTYPE=0 (stored)
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes()); // NLEN
        out.extend_from_slice(&data[pos..end]);
        pos = end;
    }
    // empty last block if data was empty or exact multiple
    if data.is_empty() {
        out.extend_from_slice(&[0x01, 0x00, 0x00, 0xFF, 0xFF]);
    }

    // Adler-32 checksum (zlib trailer)
    let (s1, s2) = adler32(data);
    let adler = ((s2 as u32) << 16) | (s1 as u32);
    out.extend_from_slice(&adler.to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> (u32, u32) {
    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    for &b in data {
        s1 = (s1 + b as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    (s1, s2)
}

fn crc32(tag: &[u8], data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in tag.iter().chain(data.iter()) {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_header_signature() {
        let img = Image::new(4, 4, [128, 64, 32]);
        let bytes = img.encode_png();
        assert_eq!(
            &bytes[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    #[test]
    fn png_encoding_is_deterministic() {
        let img = Image::new(8, 8, [255, 0, 128]);
        assert_eq!(img.encode_png(), img.encode_png());
    }

    #[test]
    fn png_ihdr_dimensions() {
        let img = Image::new(16, 12, [0, 0, 0]);
        let bytes = img.encode_png();
        // IHDR data starts at offset 16 (8 sig + 4 len + 4 tag)
        let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        assert_eq!((w, h), (16, 12));
    }
}
