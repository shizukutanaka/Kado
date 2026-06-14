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

    // ── ローカルファイルヘッダ + データ ──
    for (name, data) in entries {
        offsets.push(out.len() as u32);
        let crc = crc32(data);
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
        let crc = crc32(data);
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
}
