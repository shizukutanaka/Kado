//! RGB 画像バッファと決定的 PNG エンコーダ。
//!
//! 外部依存ゼロ。IDAT は `deflate` モジュール (非公開) の RLE限定 DEFLATE で圧縮する
//! (問281)。≤2秒の screenshot KPI は圧縮ありでも達成できる (問7)。
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

    /// `factor`×`factor` ブロック平均でダウンサンプルする (SSAA 用, 問56)。
    /// 整数平均ゆえ決定的 (問5)。`width`/`height` は `factor` で割り切れる前提
    /// (呼び出し側が保証する)。`factor <= 1` は等倍コピー。
    pub fn downsample(&self, factor: usize) -> Image {
        if factor <= 1 {
            return Image {
                width: self.width,
                height: self.height,
                pixels: self.pixels.clone(),
            };
        }
        let ow = self.width / factor;
        let oh = self.height / factor;
        let mut out = Image::new(ow, oh, [0, 0, 0]);
        let n = (factor * factor) as u32;
        for oy in 0..oh {
            for ox in 0..ow {
                let mut acc = [0u32; 3];
                for dy in 0..factor {
                    for dx in 0..factor {
                        let off = ((oy * factor + dy) * self.width + (ox * factor + dx)) * 3;
                        acc[0] += self.pixels[off] as u32;
                        acc[1] += self.pixels[off + 1] as u32;
                        acc[2] += self.pixels[off + 2] as u32;
                    }
                }
                out.set(
                    ox,
                    oy,
                    [(acc[0] / n) as u8, (acc[1] / n) as u8, (acc[2] / n) as u8],
                );
            }
        }
        out
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

        // スキャンラインに PNG 行フィルタを適用 (問287)。None(0)/Sub(1)/Up(2) を
        // 各行で試し、絶対値和 (符号付き解釈) 最小のものを決定的に選ぶ。平坦背景や
        // 陰影グラデーションは差分化でゼロ連長に変わり、問281 の RLE で強く縮む。
        // 退行防止 (問281 の構造保証の踏襲): 全 None のバイト列も別途圧縮し、
        // 適応フィルタが小さくならなければ None 版を採用する——旧実装 (常に None) より
        // 悪化することは構造的にあり得ない。
        let raw_none = self.filtered_scanlines(false);
        let raw_adaptive = self.filtered_scanlines(true);
        let idat_none = super::deflate::zlib_compress(&raw_none);
        let idat_adaptive = super::deflate::zlib_compress(&raw_adaptive);
        let idat = if idat_adaptive.len() < idat_none.len() {
            idat_adaptive
        } else {
            idat_none
        };
        write_chunk(&mut out, b"IDAT", &idat);

        // IEND
        write_chunk(&mut out, b"IEND", &[]);
        out
    }

    /// 全スキャンラインをフィルタして `[filter_byte, ...data]` を連結した
    /// IDAT 前バイト列を返す (問287)。`adaptive` が false なら全行 None(0) 固定
    /// (旧実装と同一)、true なら行ごとに None/Sub/Up の最小絶対値和を選ぶ。
    fn filtered_scanlines(&self, adaptive: bool) -> Vec<u8> {
        let stride = self.width * 3;
        let mut raw = Vec::with_capacity(self.height * (1 + stride));
        let mut prev: Option<&[u8]> = None;
        for row in 0..self.height {
            let cur = &self.pixels[row * stride..(row + 1) * stride];
            if adaptive {
                push_best_filter(&mut raw, cur, prev);
            } else {
                raw.push(0x00);
                raw.extend_from_slice(cur);
            }
            prev = Some(cur);
        }
        raw
    }

    pub fn write_png(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.encode_png())
    }
}

/// RGB 8bit の1ピクセルあたりバイト数 (Sub フィルタの左参照距離)。
const PNG_BPP: usize = 3;

/// PNG 行フィルタのバイトを符号付き (i8) とみなした絶対値和 (問287)。
/// PNG 仕様が推奨する最小絶対値和 (MSAD) ヒューリスティックの評価関数。
fn filter_abs_sum(bytes: &[u8]) -> u64 {
    bytes.iter().map(|&b| (b as i8).unsigned_abs() as u64).sum()
}

/// `row` に None(0)/Sub(1)/Up(2) を試し、絶対値和最小のフィルタで
/// `[filter_type, ...filtered]` を `out` に追記する (問287)。
/// タイブレークは番号の小さい順 (None < Sub < Up) で決定的。
/// 全バイト演算は wrapping (mod 256) で PNG 仕様どおり。
fn push_best_filter(out: &mut Vec<u8>, row: &[u8], prev: Option<&[u8]>) {
    let n = row.len();
    // None: そのまま。
    let none = row;
    // Sub: 左 (BPP 手前) との差。境界は 0。
    let sub: Vec<u8> = (0..n)
        .map(|i| {
            let a = if i >= PNG_BPP { row[i - PNG_BPP] } else { 0 };
            row[i].wrapping_sub(a)
        })
        .collect();
    // Up: 直上との差。先頭行は prev=None → 0 (= None と一致)。
    let up: Vec<u8> = (0..n)
        .map(|i| row[i].wrapping_sub(prev.map_or(0, |p| p[i])))
        .collect();

    let mut best_type = 0u8;
    let mut best_score = filter_abs_sum(none);
    // 番号の小さいものを優先するため、厳密不等号 `<` で更新 (タイは既存を維持)。
    let s_sub = filter_abs_sum(&sub);
    if s_sub < best_score {
        best_score = s_sub;
        best_type = 1;
    }
    let s_up = filter_abs_sum(&up);
    if s_up < best_score {
        best_type = 2;
    }
    out.push(best_type);
    match best_type {
        1 => out.extend_from_slice(&sub),
        2 => out.extend_from_slice(&up),
        _ => out.extend_from_slice(none),
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

/// Adler-32 チェックサム (zlib トレーラ・問108)。[`super::deflate`] からも参照される。
pub(super) fn adler32(data: &[u8]) -> (u32, u32) {
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

    // ── PNG 行フィルタ (問287) ────────────────────────────────────────────────

    #[test]
    fn filter_abs_sum_treats_bytes_as_signed() {
        // 0x00→0, 0x01→1, 0xFF→1 (=-1), 0x80→128 (=-128)。合計 130。
        assert_eq!(filter_abs_sum(&[0x00, 0x01, 0xFF, 0x80]), 130);
    }

    #[test]
    fn sub_filter_chosen_and_correct_for_horizontal_ramp() {
        // 横方向に一定勾配の1行 → Sub フィルタで (先頭ピクセルを除き) 一定差分になり
        // 絶対値和が None より小さくなる。BPP=3 なので各チャンネル独立に左を引く。
        // R チャンネルが 10,20,30,40、G/B は 0 固定の 4px 行。
        let row: Vec<u8> = vec![10, 0, 0, 20, 0, 0, 30, 0, 0, 40, 0, 0];
        let mut out = Vec::new();
        push_best_filter(&mut out, &row, None);
        assert_eq!(out[0], 1, "horizontal ramp must pick Sub(1)");
        // filtered: 先頭 [10,0,0]、以降は左との差 [10,0,0] ×3。
        assert_eq!(&out[1..], &[10, 0, 0, 10, 0, 0, 10, 0, 0, 10, 0, 0]);
    }

    #[test]
    fn up_filter_chosen_and_correct_for_vertical_repeat() {
        // 前行と同一の行 → Up フィルタで全ゼロになり、None より確実に小さい。
        let prev: Vec<u8> = vec![50, 60, 70, 80, 90, 100];
        let row = prev.clone();
        let mut out = Vec::new();
        push_best_filter(&mut out, &row, Some(&prev));
        assert_eq!(out[0], 2, "row identical to the one above must pick Up(2)");
        assert_eq!(&out[1..], &[0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn first_row_up_equals_none_and_ties_to_none() {
        // 先頭行 (prev=None) では Up は None と同一バイト列。幅1px (3バイト) なら
        // Sub も左参照が無く None と同一 → 3フィルタ全て同点 → タイブレークで None(0)。
        let row: Vec<u8> = vec![100, 50, 25];
        let mut out = Vec::new();
        push_best_filter(&mut out, &row, None);
        assert_eq!(out[0], 0, "an all-tie row must resolve to None(0)");
        assert_eq!(&out[1..], &row[..]);
    }

    #[test]
    fn adaptive_filtering_never_regresses_vs_none() {
        // 構造保証 (問281 の踏襲): encode_png が生成する IDAT は、常に全 None 版の
        // IDAT 以下の長さ。フラット・勾配・ランダム風のどのパターンでも成立する。
        for seed in 0u32..6 {
            let mut img = Image::new(24, 24, [200, 210, 220]);
            for y in 0..24 {
                for x in 0..24 {
                    // 決定的な擬似パターン (勾配 + 市松) — 乱数は使わない。
                    let v = ((x * 7 + y * 13 + seed as usize * 29) % 256) as u8;
                    img.set(x, y, [v, v.wrapping_add(40), 220u8.wrapping_sub(v)]);
                }
            }
            let none = super::super::deflate::zlib_compress(&img.filtered_scanlines(false));
            let produced_len = idat_len(&img.encode_png());
            assert!(
                produced_len <= none.len(),
                "seed {seed}: adaptive IDAT {produced_len} must be <= none-only IDAT {}",
                none.len()
            );
        }
    }

    #[test]
    fn repeated_rows_strictly_smaller_with_filter() {
        // 全行が同一の横グラデーション。問281 の RLE は距離 {1,3} 限定で行をまたいだ
        // 参照 (距離 = stride) ができないため None では各行を圧縮しきれない。Up フィルタは
        // 2 行目以降を全ゼロ化する → IDAT が確実に小さくなる (フィルタの実利益の証明)。
        let mut img = Image::new(32, 64, [0, 0, 0]);
        for y in 0..64 {
            for x in 0..32 {
                let v = (x * 8) as u8; // 行内は横方向の勾配、全行で同一。
                img.set(x, y, [v, 255u8.wrapping_sub(v), v / 2]);
            }
        }
        let none = super::super::deflate::zlib_compress(&img.filtered_scanlines(false)).len();
        let adaptive = super::super::deflate::zlib_compress(&img.filtered_scanlines(true)).len();
        assert!(
            adaptive < none,
            "repeated rows must compress smaller via Up filter: adaptive={adaptive} none={none}"
        );
    }

    /// テスト用: PNG バイト列から最初の IDAT チャンクの本体長を取り出す。
    fn idat_len(png: &[u8]) -> usize {
        let mut i = 8; // シグネチャをスキップ。
        loop {
            let len = u32::from_be_bytes(png[i..i + 4].try_into().unwrap()) as usize;
            let tag = &png[i + 4..i + 8];
            if tag == b"IDAT" {
                return len;
            }
            i += 12 + len; // len(4)+tag(4)+data(len)+crc(4)
        }
    }

    #[test]
    fn downsample_averages_blocks() {
        // 問56: 2×2 を factor=2 でダウンサンプルすると 1 画素に平均化される。
        let mut img = Image::new(2, 2, [0, 0, 0]);
        img.set(0, 0, [100, 0, 0]);
        img.set(1, 0, [0, 100, 0]);
        img.set(0, 1, [0, 0, 100]);
        img.set(1, 1, [40, 40, 40]);
        let ds = img.downsample(2);
        assert_eq!((ds.width, ds.height), (1, 1));
        // 各チャネル平均: R=(100+0+0+40)/4=35, G=(0+100+0+40)/4=35, B=(0+0+100+40)/4=35。
        assert_eq!(&ds.pixels[0..3], &[35, 35, 35]);
    }

    #[test]
    fn downsample_factor_one_is_identity() {
        let mut img = Image::new(3, 2, [10, 20, 30]);
        img.set(1, 1, [200, 100, 50]);
        let ds = img.downsample(1);
        assert_eq!(ds.pixels, img.pixels);
        assert_eq!((ds.width, ds.height), (3, 2));
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

    #[test]
    fn crc32_matches_known_vectors() {
        // 問108: PNG チャンクの整合性は CRC-32 (IEEE 802.3) に依存する。値が誤ると
        // 厳格な PNG リーダがチャンクを拒否する。標準チェック値で固定する。
        // CRC-32("123456789") = 0xCBF43926 (業界標準のチェック値)。
        assert_eq!(crc32(b"", b"123456789"), 0xCBF4_3926);
        // tag+data は連結されるので分割位置に依らず同じ。
        assert_eq!(crc32(b"1234", b"56789"), 0xCBF4_3926);
        assert_eq!(crc32(b"123456789", b""), 0xCBF4_3926);
        // 空入力の CRC は 0。
        assert_eq!(crc32(b"", b""), 0x0000_0000);
    }

    #[test]
    fn adler32_matches_known_vectors() {
        // 問108: zlib トレーラの Adler-32。値が誤ると展開時に壊れたストリーム扱いになる。
        // Adler-32("123456789") = 0x091E01DE (combined = (s2<<16)|s1)。
        let (s1, s2) = adler32(b"123456789");
        assert_eq!((s1, s2), (478, 2334), "adler32 s1/s2 components");
        assert_eq!((s2 << 16) | s1, 0x091E_01DE, "adler32 combined value");
        // 空入力は (s1=1, s2=0) → combined 1。
        let (e1, e2) = adler32(b"");
        assert_eq!((e1, e2), (1, 0));
        assert_eq!((e2 << 16) | e1, 0x0000_0001);
    }

    #[test]
    fn downsample_factor_four_averages_sixteen_pixels() {
        // 問197: downsample_averages_blocks は factor=2 のみ確認。
        // factor=4 では 4×4=16 画素を平均する (n=16 の除算) を確認する。
        // (factor*factor) の除算が 4 を超えてもオーバーフローしないことも固定。
        let mut img = Image::new(4, 4, [0, 0, 0]);
        // 16 画素の R チャンネルを 0, 16, 32, ..., 240 に設定。
        // 合計 = 16 * (0+1+...+15) = 16 * 120 = 1920; 平均 = 1920/16 = 120。
        for i in 0..16 {
            let x = i % 4;
            let y = i / 4;
            img.set(x, y, [(i * 16) as u8, 128, 64]);
        }
        let ds = img.downsample(4);
        assert_eq!((ds.width, ds.height), (1, 1), "4×4 → 1×1 output");
        assert_eq!(
            ds.pixels[0], 120,
            "R avg of 0..240 step 16 = 120, got {}",
            ds.pixels[0]
        );
        assert_eq!(
            ds.pixels[1], 128,
            "G must be constant 128, got {}",
            ds.pixels[1]
        );
        assert_eq!(
            ds.pixels[2], 64,
            "B must be constant 64, got {}",
            ds.pixels[2]
        );
    }
}
