//! 決定的 DEFLATE エンコーダ (RFC 1951, BTYPE=1 固定 Huffman・RLE限定・問281)。
//!
//! 外部依存ゼロ (ADR-003)。汎用 LZ77 (任意距離の後方参照探索) は実装量・検証量が
//! 大きいため、Kado の実際のワークロード (PNG の RGB 画素列は screenshot の単色
//! 背景・平坦面が大半を占める) に絞り、**固定候補距離 {1, 3} のみ**の後方参照を
//! 一致として検出する。distance=1 は「同一バイトの連続」(グレー系で R=G=B の場合等)、
//! distance=3 は「同一 RGB 画素値 (3バイト) の連続」(R≠G≠B の一般的な単色背景・面)を
//! 捉える——後者が実際の画像データでは支配的 (問281 実測で発見: distance=1 限定では
//! R≠G≠B の背景色を全く圧縮できなかった)。これは RFC 1951 準拠の正当な DEFLATE
//! ストリームであり、標準の zlib/PNG デコーダでそのまま展開できる——独自フォーマットではない。
//!
//! 対応しないもの (意図的な非対応・SPEC §9 と同じ「正直な契約」の精神):
//!   - 距離 {1,3} 以外の一般的な LZ77 一致 (任意間隔の繰り返しパターンは圧縮されない)
//!   - BTYPE=2 (動的 Huffman): 頻度統計に基づく符号長最適化は行わない
//!
//! これらを実装すればさらに圧縮率は上がるが、複雑さとのトレードオフで見送る
//! (`docs/socratic-review.md` 問276 の thread/helix と同じ判断軸)。
//!
//! **固定 Huffman の既知の弱点と対策 (問281 実測で発見)**: 固定 Huffman のリテラル表は
//! 値144-255に9bit (0-143は8bit) を割り当てるため、陰影のある3D形状の表面のように
//! リテラルが支配的で繰り返しの少ない領域では、raw (8bit/byte) より**膨張しうる**。
//! このため `zlib_compress` は RLE 圧縮と無圧縮 (stored ブロック) の両方を計算し、
//! **小さい方を採用する** (決定的な比較・DEFLATE 仕様が許容する標準的な手法)。
//! これにより新実装が旧実装 (常に stored) より悪化することは構造的にあり得ない。

/// `data` を zlib ストリーム (2バイトヘッダ + deflate 本体 + Adler-32 トレーラ) に
/// 圧縮する。RLE (固定 Huffman) と無圧縮 (stored) の両方を計算し、**小さい方**を
/// 採用する (問281: 固定 Huffman はリテラルが支配的なデータでは膨張しうるため、
/// stored へのフォールバックで新実装が旧実装より悪化しないことを構造的に保証する)。
pub fn zlib_compress(data: &[u8]) -> Vec<u8> {
    let rle_body = deflate_rle(data);
    let stored_body = deflate_stored(data);
    let body = if rle_body.len() <= stored_body.len() {
        rle_body
    } else {
        stored_body
    };

    let mut out = Vec::with_capacity(body.len() + 6);
    // zlib ヘッダ: CMF=0x78 (deflate, window=32KB), FLG=0x01
    // (CMF*256+FLG が31の倍数という zlib の要件を満たす既存値をそのまま使用)。
    out.push(0x78);
    out.push(0x01);
    out.extend_from_slice(&body);
    let (s1, s2) = super::image::adler32(data);
    let adler = (s2 << 16) | s1;
    out.extend_from_slice(&adler.to_be_bytes());
    out
}

/// 無圧縮 (BTYPE=0, stored) の deflate 本体。65535 バイトごとにブロック分割する
/// (RFC 1951 §3.2.4 の LEN フィールドが16bitのため)。RLE が不利な場合のフォールバック。
fn deflate_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 65535 * 5 + 5);
    let mut pos = 0;
    loop {
        let end = (pos + 65535).min(data.len());
        let bfinal = if end == data.len() { 1u8 } else { 0u8 };
        let len = (end - pos) as u16;
        out.push(bfinal); // BFINAL, BTYPE=00 (stored) はビット1-2が0なのでbfinalのみで足りる
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes()); // NLEN
        out.extend_from_slice(&data[pos..end]);
        pos = end;
        if bfinal == 1 {
            break;
        }
    }
    out
}

/// RLE 限定の DEFLATE 本体 (BTYPE=1 固定 Huffman・単一ブロック・BFINAL=1)。
/// stored ブロック (旧実装) と違い 65535 バイトの分割上限が無いため常に1ブロック。
/// 候補距離: 1 (同一バイト連続)・3 (同一 RGB 画素値連続)。問281 実測で発見した
/// 「distance=1 限定では R≠G≠B の背景色を圧縮できない」を解消する最小限の拡張。
const CANDIDATE_DISTANCES: [usize; 2] = [1, 3];

/// 位置 `i` から始まる最長一致を候補距離の中から探す。LZ77 の自己重複コピー
/// (一致長が距離を超えてもよい。参照元がコピー先の埋まった部分を指す) を許容する
/// ため、`data[i+k] == data[i+k-dist]` を単純に伸ばすだけで正しく動作する。
fn find_best_match(data: &[u8], i: usize) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None; // (distance, length)
    for &dist in &CANDIDATE_DISTANCES {
        if i < dist {
            continue; // 参照先がまだ存在しない (出力開始直後)。
        }
        let mut len = 0usize;
        while i + len < data.len() && len < 258 && data[i + len] == data[i + len - dist] {
            len += 1;
        }
        if len >= 3 {
            let better = match best {
                None => true,
                Some((_, blen)) => len > blen,
            };
            if better {
                best = Some((dist, len));
            }
        }
    }
    best
}

fn deflate_rle(data: &[u8]) -> Vec<u8> {
    let mut bw = BitWriter::new();
    bw.write_bits(1, 1); // BFINAL=1
    bw.write_bits(1, 2); // BTYPE=01 (固定 Huffman)

    let mut i = 0;
    while i < data.len() {
        match find_best_match(data, i) {
            Some((dist, len)) => {
                write_match(&mut bw, len, dist);
                i += len;
            }
            None => {
                write_literal(&mut bw, data[i]);
                i += 1;
            }
        }
    }
    write_end_of_block(&mut bw);
    bw.finish()
}

// ── ビット単位の出力 (LSB-first バイトパッキング, RFC 1951 §3.1.1) ─────────────

struct BitWriter {
    bytes: Vec<u8>,
    acc: u32,
    nbits: u8,
}

impl BitWriter {
    fn new() -> Self {
        BitWriter {
            bytes: Vec::new(),
            acc: 0,
            nbits: 0,
        }
    }

    /// `value` の下位 `n` ビットを LSB-first で書く (n <= 16 前提)。
    fn write_bits(&mut self, value: u32, n: u8) {
        debug_assert!(n <= 16);
        let mask = if n == 32 { u32::MAX } else { (1u32 << n) - 1 };
        self.acc |= (value & mask) << self.nbits;
        self.nbits += n;
        while self.nbits >= 8 {
            self.bytes.push((self.acc & 0xFF) as u8);
            self.acc >>= 8;
            self.nbits -= 8;
        }
    }

    /// Huffman 符号 (慣習的に MSB-first で表記される `code`/`len`) を書く。
    /// ビット順を反転してから `write_bits` (LSB-first) へ渡す (RFC 1951 §3.2.2 の
    /// 「シンボルの Huffman 符号だけは他のフィールドと逆に MSB-first で詰める」規則)。
    fn write_huffman(&mut self, code: u16, len: u8) {
        let mut rev: u32 = 0;
        for i in 0..len {
            rev |= (((code >> i) & 1) as u32) << (len - 1 - i);
        }
        self.write_bits(rev, len);
    }

    fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            self.bytes.push((self.acc & 0xFF) as u8);
        }
        self.bytes
    }
}

// ── 固定 Huffman 符号表 (RFC 1951 §3.2.6) ──────────────────────────────────────

/// リテラルバイト (0-255) を固定 Huffman 符号で書く。
fn write_literal(bw: &mut BitWriter, byte: u8) {
    let sym = byte as u32;
    if sym <= 143 {
        bw.write_huffman((0x30 + sym) as u16, 8);
    } else {
        bw.write_huffman((0x190 + (sym - 144)) as u16, 9);
    }
}

/// end-of-block シンボル (256): 7 bit, code=0000000。
fn write_end_of_block(bw: &mut BitWriter) {
    write_length_literal_symbol(bw, 256);
}

/// 長さ/リテラル木のシンボル 256-287 を固定 Huffman 符号で書く
/// (256-279: 7 bit code=symbol-256／280-287: 8 bit code=0xC0+(symbol-280))。
fn write_length_literal_symbol(bw: &mut BitWriter, symbol: u32) {
    if symbol <= 279 {
        bw.write_huffman((symbol - 256) as u16, 7);
    } else {
        bw.write_huffman((0xC0 + (symbol - 280)) as u16, 8);
    }
}

/// 長さ符号の基準値と追加ビット数のテーブル (RFC 1951 §3.2.5)。
/// index = symbol - 257。
const LENGTH_TABLE: [(u16, u8); 29] = [
    (3, 0),
    (4, 0),
    (5, 0),
    (6, 0),
    (7, 0),
    (8, 0),
    (9, 0),
    (10, 0),
    (11, 1),
    (13, 1),
    (15, 1),
    (17, 1),
    (19, 2),
    (23, 2),
    (27, 2),
    (31, 2),
    (35, 3),
    (43, 3),
    (51, 3),
    (59, 3),
    (67, 4),
    (83, 4),
    (99, 4),
    (115, 4),
    (131, 5),
    (163, 5),
    (195, 5),
    (227, 5),
    (258, 0),
];

/// `len` (3..=258) に対応する (symbol, extra_bits, extra_value) を求める。
fn length_code(len: usize) -> (u32, u8, u32) {
    for idx in (0..LENGTH_TABLE.len()).rev() {
        let (base, extra) = LENGTH_TABLE[idx];
        if len as u16 >= base {
            return (257 + idx as u32, extra, (len as u32) - base as u32);
        }
    }
    unreachable!("length must be >= 3, got {len}")
}

/// 長さ `len` (3..=258)・距離 `distance` (CANDIDATE_DISTANCES のいずれか) の
/// 一致を書く。距離符号は固定 Huffman で常に5 bit (RFC 1951 §3.2.6)。
/// RFC 1951 §3.2.5 の距離符号表: code 0→距離1 (extra=0), code 2→距離3 (extra=0)。
fn write_match(bw: &mut BitWriter, len: usize, distance: usize) {
    let (symbol, extra_bits, extra_value) = length_code(len);
    write_length_literal_symbol(bw, symbol);
    if extra_bits > 0 {
        bw.write_bits(extra_value, extra_bits);
    }
    let dist_code: u16 = match distance {
        1 => 0,
        3 => 2,
        _ => unreachable!("only distances in CANDIDATE_DISTANCES are supported, got {distance}"),
    };
    bw.write_huffman(dist_code, 5);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 検証用インフレータ (BTYPE=0/1 のみ対応。問281: 外部ツールに頼らず
    //    自己完結で round-trip 検証するための最小デコーダ。RFC 1951 準拠の
    //    汎用実装であり、自エンコーダの出力に限定した特化デコーダではない
    //    (「自分の書いたバグに同じ形でしか気付けないデコーダ」を避けるため)。──

    struct BitReader<'a> {
        data: &'a [u8],
        byte_pos: usize,
        bit_pos: u8,
    }

    impl<'a> BitReader<'a> {
        fn new(data: &'a [u8]) -> Self {
            BitReader {
                data,
                byte_pos: 0,
                bit_pos: 0,
            }
        }

        fn read_bit(&mut self) -> u32 {
            let byte = self.data[self.byte_pos];
            let bit = (byte >> self.bit_pos) & 1;
            self.bit_pos += 1;
            if self.bit_pos == 8 {
                self.bit_pos = 0;
                self.byte_pos += 1;
            }
            bit as u32
        }

        fn read_bits(&mut self, n: u8) -> u32 {
            let mut v = 0u32;
            for i in 0..n {
                v |= self.read_bit() << i;
            }
            v
        }

        /// Huffman 符号を1ビットずつ読み、MSB-first の符号値として蓄積する
        /// (書き込み側の bit-reversal と対称)。
        fn read_huffman_bit_msb(&mut self, code: &mut u32) {
            *code = (*code << 1) | self.read_bit();
        }

        fn align_to_byte(&mut self) {
            if self.bit_pos != 0 {
                self.bit_pos = 0;
                self.byte_pos += 1;
            }
        }
    }

    /// 固定 Huffman の長さ/リテラル木からシンボル (0-287) を1つ復号する。
    fn decode_length_literal_symbol(br: &mut BitReader) -> u32 {
        let mut code = 0u32;
        for len in 1..=9u8 {
            br.read_huffman_bit_msb(&mut code);
            match len {
                7 => {
                    if code <= 0b0010111 {
                        return 256 + code;
                    }
                }
                8 => {
                    if (0x30..=0xBF).contains(&code) {
                        return code - 0x30;
                    }
                    if (0xC0..=0xC7).contains(&code) {
                        return 280 + (code - 0xC0);
                    }
                }
                9 => {
                    if (0x190..=0x1FF).contains(&code) {
                        return 144 + (code - 0x190);
                    }
                }
                _ => {}
            }
        }
        panic!("invalid fixed-huffman length/literal code");
    }

    /// 固定 Huffman の距離木からシンボル (0-29) を1つ復号する (常に5 bit)。
    fn decode_distance_symbol(br: &mut BitReader) -> u32 {
        let mut code = 0u32;
        for _ in 0..5 {
            br.read_huffman_bit_msb(&mut code);
        }
        code
    }

    const DIST_TABLE: [(u32, u8); 30] = [
        (1, 0),
        (2, 0),
        (3, 0),
        (4, 0),
        (5, 1),
        (7, 1),
        (9, 2),
        (13, 2),
        (17, 3),
        (25, 3),
        (33, 4),
        (49, 4),
        (65, 5),
        (97, 5),
        (129, 6),
        (193, 6),
        (257, 7),
        (385, 7),
        (513, 8),
        (769, 8),
        (1025, 9),
        (1537, 9),
        (2049, 10),
        (3073, 10),
        (4097, 11),
        (6145, 11),
        (8193, 12),
        (12289, 12),
        (16385, 13),
        (24577, 13),
    ];

    fn inflate(data: &[u8]) -> Vec<u8> {
        let mut br = BitReader::new(data);
        let mut out: Vec<u8> = Vec::new();
        loop {
            let bfinal = br.read_bits(1);
            let btype = br.read_bits(2);
            match btype {
                0 => {
                    // stored block: バイト境界に揃え、LEN/NLEN を読んでコピー。
                    br.align_to_byte();
                    let len = br.read_bits(16) as usize;
                    let _nlen = br.read_bits(16);
                    for _ in 0..len {
                        out.push(br.data[br.byte_pos] as u8);
                        br.byte_pos += 1;
                    }
                }
                1 => loop {
                    let symbol = decode_length_literal_symbol(&mut br);
                    if symbol < 256 {
                        out.push(symbol as u8);
                    } else if symbol == 256 {
                        break;
                    } else {
                        let (base, extra) = LENGTH_TABLE[(symbol - 257) as usize];
                        let len = base as u32 + br.read_bits(extra);
                        let dsym = decode_distance_symbol(&mut br) as usize;
                        let (dbase, dextra) = DIST_TABLE[dsym];
                        let dist = dbase + br.read_bits(dextra);
                        let start = out.len() - dist as usize;
                        for k in 0..len as usize {
                            let b = out[start + k];
                            out.push(b);
                        }
                    }
                },
                other => panic!("unsupported BTYPE {other} in test decoder"),
            }
            if bfinal == 1 {
                break;
            }
        }
        out
    }

    fn zlib_inflate(zlib_bytes: &[u8]) -> Vec<u8> {
        // 2バイトヘッダをスキップし、末尾4バイト (Adler-32) を除いた本体を渡す。
        inflate(&zlib_bytes[2..zlib_bytes.len() - 4])
    }

    #[test]
    fn roundtrip_empty() {
        let data: &[u8] = &[];
        let z = zlib_compress(data);
        assert_eq!(zlib_inflate(&z), data);
    }

    #[test]
    fn roundtrip_single_byte() {
        let data: &[u8] = &[42];
        let z = zlib_compress(data);
        assert_eq!(zlib_inflate(&z), data);
    }

    #[test]
    fn roundtrip_short_norun() {
        // 3バイト未満の繰り返ししかない → 全てリテラル経路。
        let data: &[u8] = b"AB";
        let z = zlib_compress(data);
        assert_eq!(zlib_inflate(&z), data);
    }

    #[test]
    fn roundtrip_long_run_of_identical_bytes() {
        // 単色 screenshot 背景を模した長い同一バイト列 (RLE の主対象)。
        let data = vec![0x2Cu8; 10_000];
        let z = zlib_compress(&data);
        assert_eq!(zlib_inflate(&z), data);
        // 実際に圧縮されていること (stored 相当の 10000+overhead より十分小さい)。
        assert!(
            z.len() < 200,
            "10000 identical bytes must compress far below stored size, got {}",
            z.len()
        );
    }

    #[test]
    fn roundtrip_run_exactly_258_and_boundary_lengths() {
        // 一致長上限 258 ちょうど、および 259 (259=258+1 でチャンク分割が要る境界)。
        for run_len in [2usize, 3, 4, 257, 258, 259, 258 * 2, 258 * 3 + 5] {
            let data = vec![0x7Bu8; run_len];
            let z = zlib_compress(&data);
            assert_eq!(zlib_inflate(&z), data, "run_len={run_len}");
        }
    }

    #[test]
    fn roundtrip_repeated_rgb_triple_distance_3() {
        // 問281: 実際の screenshot 背景の典型例——R≠G≠B の RGB 画素値 (3バイト) が
        // 連続する (distance=1 では検出できず distance=3 が必要)。
        let mut data = Vec::new();
        for _ in 0..2000 {
            data.extend_from_slice(&[40u8, 44, 52]); // Kado 既定背景色
        }
        let z = zlib_compress(&data);
        assert_eq!(zlib_inflate(&z), data, "round-trip must be correct");
        assert!(
            z.len() < 200,
            "6000 bytes of repeated RGB triple must compress far below raw size, got {}",
            z.len()
        );
    }

    #[test]
    fn distance_3_beats_distance_1_for_rgb_background() {
        // 問281 (実測で発見したバグの直接的な回帰テスト): distance=1 限定の旧実装は
        // R≠G≠B の背景を全く圧縮できなかった。find_best_match が distance=3 を
        // 正しく選ぶことで、実際に圧縮されることを確認する。
        let mut data = Vec::new();
        for _ in 0..1000 {
            data.extend_from_slice(&[40u8, 44, 52]);
        }
        let rle = deflate_rle(&data);
        let stored = deflate_stored(&data);
        assert!(
            rle.len() < stored.len(),
            "RLE with distance=3 must beat stored for RGB-triple-repeated data: rle={} stored={}",
            rle.len(),
            stored.len()
        );
    }

    #[test]
    fn roundtrip_mixed_literals_and_runs() {
        // リテラル・短い繰り返し (run<3)・長い繰り返し (run>=3) が混在するデータ。
        let mut data = Vec::new();
        data.extend_from_slice(b"Kado");
        data.extend_from_slice(&[9u8; 5]);
        data.extend_from_slice(b"XY");
        data.extend_from_slice(&[200u8; 1000]);
        data.extend_from_slice(&[1u8, 2, 1, 2, 1]);
        let z = zlib_compress(&data);
        assert_eq!(zlib_inflate(&z), data);
    }

    #[test]
    fn roundtrip_all_256_byte_values_as_literals() {
        // 全リテラルシンボル (0-255) を1回ずつ含む列。8bit/9bit 符号の境界
        // (143/144) を含む固定 Huffman 表の全域を運動させる。
        let data: Vec<u8> = (0..=255u8).collect();
        let z = zlib_compress(&data);
        assert_eq!(zlib_inflate(&z), data);
    }

    #[test]
    fn compression_is_deterministic() {
        let data = vec![7u8; 5000];
        assert_eq!(zlib_compress(&data), zlib_compress(&data));
    }

    #[test]
    fn falls_back_to_stored_when_rle_would_expand() {
        // 問281 (実測発見): 値144-255 (固定Huffmanで9bit) が連続かつ繰り返しなしで
        // 並ぶデータは RLE が一致を見つけられず、全リテラルが9bit符号化されて
        // raw (8bit/byte) より膨張しうる。zlib_compress は stored へフォールバック
        // することで、この場合でも stored 相当以下のサイズに収まることを固定する。
        let data: Vec<u8> = (0..2000u32).map(|i| (144 + (i * 37) % 112) as u8).collect();
        let z = zlib_compress(&data);
        assert_eq!(zlib_inflate(&z), data, "round-trip must still be correct");
        // stored 相当の理論サイズ: zlibヘッダ2 + deflate_stored本体 + adler4。
        let stored_equivalent = 2 + deflate_stored(&data).len() + 4;
        assert!(
            z.len() <= stored_equivalent,
            "must never exceed the stored-block fallback size: got {}, stored={}",
            z.len(),
            stored_equivalent
        );
    }

    #[test]
    fn zlib_header_bytes_are_valid() {
        let z = zlib_compress(b"test");
        assert_eq!(z[0], 0x78);
        assert_eq!(z[1], 0x01);
        // CMF*256+FLG は 31 の倍数でなければならない (zlib 仕様必須条件)。
        let check = (z[0] as u32) * 256 + z[1] as u32;
        assert_eq!(check % 31, 0, "zlib header FCHECK constraint violated");
    }
}
