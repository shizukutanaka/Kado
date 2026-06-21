//! 最小 ZIP (STORED / 無圧縮) 書き出し。
//!
//! OPC パッケージ (3MF など) の容器として用いる。決定的 (問5):
//! タイムスタンプ固定 (0)・エントリ順保存・std のみ (ADR-003 / 問4)。
//! 無圧縮 STORED ゆえ compressed size == uncompressed size。

// ZIP シグネチャ (u32 LE)。
const SIG_LOCAL: u32 = 0x0403_4b50; // "PK\x03\x04"
const SIG_CENTRAL: u32 = 0x0201_4b50; // "PK\x01\x02"
const SIG_EOCD: u32 = 0x0605_4b50; // "PK\x05\x06"
const VERSION: u16 = 20; // 2.0

/// `(エントリ名, データ)` の列を STORED ZIP バイト列に組み立てる。
pub fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut offsets = Vec::with_capacity(entries.len());
    // 問120: CRC を第1パスでキャッシュし、第2パス (中央ディレクトリ) で再利用する。
    // 旧実装は各エントリで crc32 を2回呼んでいた。大きな 3MF では無駄な倍計算になる上、
    // ローカルヘッダと中央ディレクトリの CRC を別ループで独立計算すると将来の変更で
    // 不一致が生じやすい。一か所で計算して共有することで一貫性を構造的に保証する。
    let crcs: Vec<u32> = entries.iter().map(|(_, data)| crc32(data)).collect();

    // ── ローカルファイルヘッダ + データ ──
    for (i, (name, data)) in entries.iter().enumerate() {
        offsets.push(out.len() as u32);
        let crc = crcs[i];
        out.extend_from_slice(&SIG_LOCAL.to_le_bytes());
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // 汎用フラグ
        out.extend_from_slice(&0u16.to_le_bytes()); // 圧縮法 = 0 (stored)
        out.extend_from_slice(&0u16.to_le_bytes()); // 更新時刻 (固定)
        out.extend_from_slice(&0u16.to_le_bytes()); // 更新日付 (固定)
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes()); // 圧縮後サイズ
        out.extend_from_slice(&(data.len() as u32).to_le_bytes()); // 展開後サイズ
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra 長
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
    }

    // ── 中央ディレクトリ ──
    let cd_start = out.len() as u32;
    let mut central = Vec::new();
    for (i, (name, data)) in entries.iter().enumerate() {
        let crc = crcs[i]; // キャッシュ済み (ローカルヘッダと同値を保証)
        central.extend_from_slice(&SIG_CENTRAL.to_le_bytes());
        central.extend_from_slice(&VERSION.to_le_bytes()); // version made by
        central.extend_from_slice(&VERSION.to_le_bytes()); // version needed
        central.extend_from_slice(&0u16.to_le_bytes()); // フラグ
        central.extend_from_slice(&0u16.to_le_bytes()); // 圧縮法
        central.extend_from_slice(&0u16.to_le_bytes()); // 時刻
        central.extend_from_slice(&0u16.to_le_bytes()); // 日付
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra 長
        central.extend_from_slice(&0u16.to_le_bytes()); // コメント長
        central.extend_from_slice(&0u16.to_le_bytes()); // 開始ディスク
        central.extend_from_slice(&0u16.to_le_bytes()); // 内部属性
        central.extend_from_slice(&0u32.to_le_bytes()); // 外部属性
        central.extend_from_slice(&offsets[i].to_le_bytes()); // ローカルヘッダ位置
        central.extend_from_slice(name.as_bytes());
    }
    let cd_size = central.len() as u32;
    out.extend_from_slice(&central);

    // ── 中央ディレクトリ終端レコード (EOCD) ──
    let n = entries.len() as u16;
    out.extend_from_slice(&SIG_EOCD.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // このディスク番号
    out.extend_from_slice(&0u16.to_le_bytes()); // CD 開始ディスク
    out.extend_from_slice(&n.to_le_bytes()); // このディスクのエントリ数
    out.extend_from_slice(&n.to_le_bytes()); // 総エントリ数
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // コメント長
    out
}

/// CRC-32 (IEEE 802.3, 多項式 0xEDB88320)。ZIP/PNG 共通。
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
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
    fn zip_has_local_and_eocd_signatures() {
        let z = build_zip(&[("a.txt", b"hello"), ("b/c.txt", b"world")]);
        // 先頭はローカルファイルヘッダ署名 "PK\x03\x04"。
        assert_eq!(&z[0..4], &[0x50, 0x4B, 0x03, 0x04]);
        // EOCD 署名がどこかに存在する。
        assert!(
            z.windows(4).any(|w| w == [0x50, 0x4B, 0x05, 0x06]),
            "EOCD signature must be present"
        );
        // エントリ名が含まれる。
        assert!(z.windows(7).any(|w| w == b"b/c.txt"));
    }

    #[test]
    fn zip_is_deterministic() {
        let e: &[(&str, &[u8])] = &[("x", b"123"), ("y", b"456")];
        assert_eq!(build_zip(e), build_zip(e));
    }

    #[test]
    fn eocd_reports_entry_count() {
        let z = build_zip(&[("a", b"1"), ("b", b"2"), ("c", b"3")]);
        // EOCD の総エントリ数フィールド (署名 +10 バイト目から u16 LE)。
        let pos = z
            .windows(4)
            .position(|w| w == [0x50, 0x4B, 0x05, 0x06])
            .unwrap();
        let total = u16::from_le_bytes(z[pos + 10..pos + 12].try_into().unwrap());
        assert_eq!(total, 3);
    }

    #[test]
    fn crc32_matches_known_vectors() {
        // 問108: 3MF (ZIP) エントリの CRC-32。値が誤ると展開ツール/スライサが
        // アーカイブを破損扱いする。標準チェック値で固定する。
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0x0000_0000);
        // PNG 側 (render::image) の crc32 と同一アルゴリズムであることの間接確認:
        // 同じ入力で同じ値になる (実装は重複だが仕様一致)。
    }

    #[test]
    fn local_header_and_central_directory_crc_are_identical() {
        // 問120: ローカルヘッダと中央ディレクトリの CRC フィールドは一致しなければ
        // ZIP として無効になりスライサ等が展開を拒否する。CRC 計算をキャッシュして
        // 共有したことで構造的に一致が保証されるが、実際のバイト列で確認する。
        //
        // ZIP 構造 (STORED, 2 エントリ):
        //   [LFH][data][LFH][data] [CD1][CD2] [EOCD]
        // LFH の CRC オフセット: 署名4+version2+flag2+method2+time2+date2 = +14
        // CD の CRC オフセット:  署名4+ver2+ver2+flag2+method2+time2+date2 = +16
        let entries: &[(&str, &[u8])] = &[
            ("a.xml", b"<hello/>"),
            ("b.bin", b"\x00\x01\x02\x03\xff"),
        ];
        let z = build_zip(entries);

        // エントリ 0 のローカルヘッダ先頭 = 0。
        let lfh0_crc = u32::from_le_bytes(z[14..18].try_into().unwrap());
        // エントリ 1 のローカルヘッダ先頭 = LFH0(30 + name0.len() + data0.len())。
        let lfh0_size = 30 + 5 + 8; // name "a.xml"=5, data=8
        let lfh1_crc = u32::from_le_bytes(z[lfh0_size + 14..lfh0_size + 18].try_into().unwrap());

        // 中央ディレクトリは EOCD から遡る。
        let eocd_pos = z.windows(4).rposition(|w| w == [0x50, 0x4B, 0x05, 0x06]).unwrap();
        let cd_start = u32::from_le_bytes(z[eocd_pos + 16..eocd_pos + 20].try_into().unwrap()) as usize;

        // CD エントリ 0 (先頭)。
        let cd0_crc = u32::from_le_bytes(z[cd_start + 16..cd_start + 20].try_into().unwrap());
        // CD エントリ 1 (46 バイト後 + name0.len() = 5)。
        let cd0_size = 46 + 5; // name "a.xml"=5
        let cd1_crc = u32::from_le_bytes(z[cd_start + cd0_size + 16..cd_start + cd0_size + 20].try_into().unwrap());

        assert_eq!(lfh0_crc, cd0_crc, "entry 0: local header CRC must match central directory CRC");
        assert_eq!(lfh1_crc, cd1_crc, "entry 1: local header CRC must match central directory CRC");

        // CRCs は独立に計算した正解と一致する。
        assert_eq!(lfh0_crc, crc32(b"<hello/>"));
        assert_eq!(lfh1_crc, crc32(b"\x00\x01\x02\x03\xff"));
    }

    #[test]
    fn central_directory_offsets_point_to_valid_local_headers() {
        // 問213: 各中央ディレクトリエントリは対応するローカルファイルヘッダ (LFH) の
        // バイトオフセットを格納する (CD の +42)。オフセットが誤ると展開ツールが
        // アーカイブを壊れ扱いする。可変長の名前を持つ 3 エントリで、各オフセットが
        // 実際に有効な LFH 署名 (PK\x03\x04) を指すことを確認する。
        // 既存テストは CRC 一致のみ確認しオフセットの正しさは未検証だった。
        let entries: &[(&str, &[u8])] = &[
            ("s.txt", b"hi"),
            ("medium_name.xml", b"<x/>"),
            ("very_long_filename_for_offset_check.bin", b"binarydata"),
        ];
        let z = build_zip(entries);

        // EOCD から中央ディレクトリ開始位置とエントリ数を取得。
        let eocd_pos = z.windows(4).rposition(|w| w == [0x50, 0x4B, 0x05, 0x06]).unwrap();
        let cd_count = u16::from_le_bytes(z[eocd_pos + 10..eocd_pos + 12].try_into().unwrap());
        assert_eq!(cd_count as usize, entries.len(), "EOCD entry count must match");
        let cd_start = u32::from_le_bytes(z[eocd_pos + 16..eocd_pos + 20].try_into().unwrap()) as usize;

        // 各 CD エントリを辿り、その LFH オフセットが有効な署名を指すことを確認。
        let mut cd_pos = cd_start;
        for (i, (name, _)) in entries.iter().enumerate() {
            // CD 署名 = PK\x01\x02。
            let cd_sig = u32::from_le_bytes(z[cd_pos..cd_pos + 4].try_into().unwrap());
            assert_eq!(cd_sig, 0x0201_4b50, "entry {i}: central directory signature");
            // CD の +42 に LFH オフセット。
            let lfh_off = u32::from_le_bytes(z[cd_pos + 42..cd_pos + 46].try_into().unwrap()) as usize;
            // そのオフセットが有効な LFH 署名 (PK\x03\x04) を指す。
            let lfh_sig = u32::from_le_bytes(z[lfh_off..lfh_off + 4].try_into().unwrap());
            assert_eq!(lfh_sig, 0x0403_4b50, "entry {i}: offset {lfh_off} must point to valid LFH");
            // LFH のファイル名がエントリ名と一致 (オフセットが正しいエントリを指す確証)。
            let name_len = u16::from_le_bytes(z[lfh_off + 26..lfh_off + 28].try_into().unwrap()) as usize;
            let lfh_name = std::str::from_utf8(&z[lfh_off + 30..lfh_off + 30 + name_len]).unwrap();
            assert_eq!(lfh_name, *name, "entry {i}: LFH name must match");
            // 次の CD エントリへ (固定 46 + 名前長)。
            cd_pos += 46 + name.len();
        }
    }
}
