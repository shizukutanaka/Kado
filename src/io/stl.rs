//! Binary STL 書き出し。
//!
//! 決定的 (問5): ヘッダ固定・リトルエンディアン f32・三角形順序はメッシュ順。
//! 同一メッシュ → バイト同一の STL。

use crate::core::Vec3;
use crate::extract::Mesh;

/// メッシュを binary STL のバイト列にエンコードする。
pub fn encode_binary(mesh: &Mesh) -> Vec<u8> {
    let mut out = Vec::with_capacity(84 + mesh.triangles.len() * 50);

    // 80バイト固定ヘッダ (決定性のため定数)。
    let mut header = [0u8; 80];
    let tag = b"kado binary stl";
    header[..tag.len()].copy_from_slice(tag);
    out.extend_from_slice(&header);

    // 三角形数 (u32 LE)。
    out.extend_from_slice(&(mesh.triangles.len() as u32).to_le_bytes());

    for t in &mesh.triangles {
        let a = mesh.vertices[t[0] as usize];
        let b = mesh.vertices[t[1] as usize];
        let c = mesh.vertices[t[2] as usize];
        let n = face_normal(a, b, c);
        for v in [n, a, b, c] {
            out.extend_from_slice(&(v.x as f32).to_le_bytes());
            out.extend_from_slice(&(v.y as f32).to_le_bytes());
            out.extend_from_slice(&(v.z as f32).to_le_bytes());
        }
        out.extend_from_slice(&0u16.to_le_bytes()); // 属性バイト数
    }
    out
}

/// メッシュを STL ファイルに書き出す。
pub fn write_binary(mesh: &Mesh, path: &std::path::Path) -> std::io::Result<()> {
    std::fs::write(path, encode_binary(mesh))
}

/// binary STL インポートのリソース上限 (SECURITY.md §4)。50 バイト/三角形なので
/// 5M 三角形 ≈ 250 MB。病的に巨大なファイルでの確保爆発を防ぐ。
pub const MAX_STL_TRIANGLES: usize = 5_000_000;

/// binary STL バイト列を [`Mesh`] へデコードする (問296)。
///
/// **設計上の制約 (ADR-001)**: インポートしたメッシュは **検証・可視化・再書き出し
/// 専用**であり、SDF シーン正本には決してしない。「SDF が唯一の正本」原則を保存する
/// ため、mesh→SDF 再構成 (Kado が避けてきた数値破綻の温床) は行わない。
///
/// 厳格に検証し、不正入力をサイレントに空メッシュへ落とさず明示エラーにする
/// (CONTRIBUTING §4 の「サイレント故障を作らない」):
/// - 84 バイト未満、宣言三角形数と実バイト長の不一致 (`84 + n*50`) を拒否する
///   (ASCII STL・切り詰め・破損はここで弾かれる)。
/// - `MAX_STL_TRIANGLES` 超過を**確保前**に拒否する (OOM 防御・SECURITY §4)。
/// - 非有限座標 (NaN/±Inf) を拒否する (問128 の NaN 伝播防止と同型)。
///
/// STL の法線フィールドは信頼できない参考値のため無視し、巻き順はファイルの
/// 頂点順に従う。頂点は [`Mesh::from_soup`] で正準キー化・重複統合され、退化三角形は
/// 落ちる。座標は STL の f32 から f64 へ広げる。
pub fn decode_binary(bytes: &[u8]) -> Result<Mesh, String> {
    if bytes.len() < 84 {
        return Err(format!(
            "binary STL too short: {} bytes < 84 (80-byte header + 4-byte count)",
            bytes.len()
        ));
    }
    let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    // 確保前に上限チェック (OOM 防御)。長さチェックより先に行う。
    if count > MAX_STL_TRIANGLES {
        return Err(format!(
            "binary STL declares {count} triangles, over the limit {MAX_STL_TRIANGLES}"
        ));
    }
    let expected = 84 + count * 50;
    if bytes.len() != expected {
        return Err(format!(
            "binary STL length mismatch: got {} bytes, but {count} triangles need {expected} \
             (ASCII STL is not supported; the file may be ASCII or truncated)",
            bytes.len()
        ));
    }
    let read_f32 = |off: usize| -> f32 {
        f32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
    };
    let mut soup: Vec<[Vec3; 3]> = Vec::with_capacity(count);
    for i in 0..count {
        // レコード = 法線12 + 頂点3×12 + 属性2 = 50 バイト。法線は読み飛ばす。
        let base = 84 + i * 50;
        let mut verts = [Vec3::ZERO; 3];
        for (v, vert) in verts.iter_mut().enumerate() {
            let off = base + 12 + v * 12;
            let (x, y, z) = (read_f32(off), read_f32(off + 4), read_f32(off + 8));
            if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                return Err(format!(
                    "binary STL triangle {i} has a non-finite vertex coordinate"
                ));
            }
            *vert = Vec3::new(x as f64, y as f64, z as f64);
        }
        soup.push(verts);
    }
    Ok(Mesh::from_soup(&soup))
}

fn face_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let n = (b - a).cross(c - a);
    let len = n.length();
    if len == 0.0 {
        Vec3::ZERO
    } else {
        n * (1.0 / len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Sdf;
    use crate::extract::polygonize;

    #[test]
    fn header_and_count_are_correct() {
        let m = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 16);
        let bytes = encode_binary(&m);
        assert_eq!(bytes.len(), 84 + m.triangles.len() * 50);
        let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap());
        assert_eq!(count as usize, m.triangles.len());
    }

    #[test]
    fn encoding_is_deterministic() {
        // 同一メッシュ → バイト同一 (問5)。
        let m = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 16);
        assert_eq!(encode_binary(&m), encode_binary(&m));
    }

    #[test]
    fn per_triangle_record_layout_round_trips_vertices() {
        // 問289: 既存テストはヘッダ・三角形数・法線までしか見ておらず、50バイトの
        // 三角形レコード (法線12 + 頂点3×12 + 属性2) のバイト配置と、書き出した頂点が
        // メッシュ頂点と一致することを独立に読み戻して検証していなかった。binary STL の
        // レイアウトを外部パーサと同じ手順で復元し、仕様適合を固定する。
        let m = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 8);
        assert!(!m.triangles.is_empty());
        let bytes = encode_binary(&m);
        let read_f32 = |off: usize| f32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
        for (i, t) in m.triangles.iter().enumerate() {
            let base = 84 + i * 50;
            // 属性バイト数 (レコード末尾 +48) は 0。
            let attr = u16::from_le_bytes(bytes[base + 48..base + 50].try_into().unwrap());
            assert_eq!(attr, 0, "tri {i}: attribute byte count must be 0");
            // 法線 (先頭12バイト) は単位長またはゼロ (退化)。
            let n = [read_f32(base), read_f32(base + 4), read_f32(base + 8)];
            let nlen = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!(
                nlen == 0.0 || (nlen - 1.0).abs() < 1e-4,
                "tri {i}: normal must be unit or zero, got len {nlen}"
            );
            // 3頂点 (法線の後、各12バイト) がメッシュ頂点と f32 精度で一致する。
            for (k, &vi) in t.iter().enumerate() {
                let voff = base + 12 + k * 12;
                let got = [read_f32(voff), read_f32(voff + 4), read_f32(voff + 8)];
                let want = m.vertices[vi as usize];
                assert!(
                    (got[0] - want.x as f32).abs() < 1e-6
                        && (got[1] - want.y as f32).abs() < 1e-6
                        && (got[2] - want.z as f32).abs() < 1e-6,
                    "tri {i} vertex {k}: STL bytes {got:?} != mesh vertex {want:?}"
                );
            }
        }
    }

    #[test]
    fn face_normal_is_unit_for_valid_triangle_and_zero_for_degenerate() {
        // 問133: face_normal は有効三角形で単位長法線、退化三角形で Vec3::ZERO を返す。
        // STL 仕様では法線はオプション/参考値だが 0 を書いてもパーサは受け入れる。
        // from_soup は重複インデックスを除去するが、共線頂点 (面積ゼロ) は除去しないため
        // face_normal の ZERO パスは実際に到達可能 (例: 手動で構築した Mesh)。
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 1.0, 0.0);
        let n = face_normal(a, b, c);
        assert!(
            (n.length() - 1.0).abs() < 1e-12,
            "valid triangle must produce unit-length normal, got length={}",
            n.length()
        );
        // XY平面の三角形の法線は +Z 方向。
        assert!(
            (n.z - 1.0).abs() < 1e-12,
            "XY-plane triangle normal must be +Z, got {n:?}"
        );

        // 退化: a, b, c が共線 → 面積 0 → 法線ゼロ。
        let degen = face_normal(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        );
        assert_eq!(
            degen,
            Vec3::ZERO,
            "collinear triangle must produce zero normal"
        );

        // 一致: 3 点が同じ → 面積 0 → 法線ゼロ。
        let coinc = face_normal(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO);
        assert_eq!(
            coinc,
            Vec3::ZERO,
            "coincident triangle must produce zero normal"
        );
    }

    #[test]
    fn face_normal_near_degenerate_is_finite_and_not_nan() {
        // 問164: len が 0 より大きいが極めて小さい (1e-150 オーダー) 三角形でも
        // 1/len が NaN/Inf にならないことを確認する。
        // 三角形の 2 辺が 1e-75 スケールだと外積長 ≈ (1e-75)^2 = 1e-150。
        // f64::MIN_POSITIVE ≈ 2.2e-308 なので 1e-150 は正規化数のまま。
        let tiny = 1e-75_f64;
        let a = Vec3::ZERO;
        let b = Vec3::new(tiny, 0.0, 0.0);
        let c = Vec3::new(0.0, tiny, 0.0);
        let n = face_normal(a, b, c);
        assert!(
            n.x.is_finite(),
            "near-degenerate: x component must be finite, got {}",
            n.x
        );
        assert!(
            n.y.is_finite(),
            "near-degenerate: y component must be finite, got {}",
            n.y
        );
        assert!(
            n.z.is_finite(),
            "near-degenerate: z component must be finite, got {}",
            n.z
        );
        // 正規化されているか 0 かどちらかでなければならない (長さが 0 でなければ単位長)。
        let len = n.length();
        assert!(
            len == 0.0 || (len - 1.0).abs() < 1e-9,
            "near-degenerate normal must be zero or unit-length, got length={len}"
        );
    }

    // ── binary STL デコード (問296, validate 専用インポート) ──────────────────

    #[test]
    fn decode_round_trips_geometry_through_encode() {
        // encode→decode で**幾何 (頂点座標)** が f32 精度で往復する。STL の法線は
        // 頂点から導出する参考値 (encode が f64 頂点から、再 encode が f32 頂点から計算する
        // ため 1ULP ずれうる) なので、法線ではなく頂点座標と三角形順序を比較する。
        let m = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 12);
        assert!(!m.triangles.is_empty());
        let bytes = encode_binary(&m);
        let decoded = decode_binary(&bytes).expect("valid STL must decode");
        assert_eq!(
            decoded.triangles.len(),
            m.triangles.len(),
            "decode must preserve triangle count"
        );
        // 各三角形の3頂点が、元メッシュの頂点を f32 に丸めた値と厳密一致する。
        for (dt, mt) in decoded.triangles.iter().zip(m.triangles.iter()) {
            for k in 0..3 {
                let dv = decoded.vertices[dt[k] as usize];
                let mv = m.vertices[mt[k] as usize];
                assert_eq!(dv.x, mv.x as f32 as f64, "vertex x must round-trip at f32");
                assert_eq!(dv.y, mv.y as f32 as f64, "vertex y must round-trip at f32");
                assert_eq!(dv.z, mv.z as f32 as f64, "vertex z must round-trip at f32");
            }
        }
        // インポートしたメッシュは水密性を検証できる (validate 経路の前提)。
        assert!(
            decoded.is_edge_manifold(),
            "a sphere STL must decode to an edge-manifold mesh"
        );
        // decode は決定的。
        assert_eq!(
            decode_binary(&bytes).unwrap().vertices,
            decoded.vertices,
            "decode must be deterministic"
        );
    }

    #[test]
    fn decode_rejects_length_mismatch_and_ascii() {
        // 宣言三角形数と実バイト長が合わない (切り詰め・ASCII STL) を明示エラーに。
        let m = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 8);
        let mut bytes = encode_binary(&m);
        bytes.truncate(bytes.len() - 10); // 末尾を欠損させる。
        assert!(
            decode_binary(&bytes)
                .unwrap_err()
                .contains("length mismatch"),
            "truncated STL must be rejected"
        );
        // "solid ..." で始まる ASCII STL 風データも長さ不一致で弾かれる。
        let ascii = b"solid teapot\nfacet normal 0 0 0\n".to_vec();
        assert!(decode_binary(&ascii).is_err(), "ASCII STL must be rejected");
        // 84 バイト未満。
        assert!(decode_binary(&[0u8; 40]).unwrap_err().contains("too short"));
    }

    #[test]
    fn decode_rejects_excessive_triangle_count_before_allocating() {
        // 巨大な宣言三角形数を、実データを持たない84バイトのヘッダだけで拒否する
        // (確保前チェック = OOM 防御)。
        let mut header = vec![0u8; 84];
        let huge = (MAX_STL_TRIANGLES as u32).saturating_add(1);
        header[80..84].copy_from_slice(&huge.to_le_bytes());
        let err = decode_binary(&header).unwrap_err();
        assert!(
            err.contains("over the limit"),
            "excessive count must be rejected before allocation, got: {err}"
        );
    }

    #[test]
    fn decode_rejects_non_finite_coordinates() {
        // 1三角形の STL を組み、頂点に NaN を仕込んで拒否されることを確認 (問128 と同型)。
        let mut bytes = vec![0u8; 84 + 50];
        bytes[80..84].copy_from_slice(&1u32.to_le_bytes());
        // 最初の頂点 x (レコード base=84, 法線12バイト後 = 96) に NaN を書く。
        bytes[96..100].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(
            decode_binary(&bytes).unwrap_err().contains("non-finite"),
            "NaN vertex must be rejected"
        );
    }

    #[test]
    fn decode_empty_mesh_stl_yields_empty_mesh() {
        // count=0 の STL (84バイト) は空メッシュへ。パニックしない。
        let empty = encode_binary(&Mesh::default());
        assert_eq!(empty.len(), 84);
        let m = decode_binary(&empty).expect("empty STL decodes");
        assert!(m.triangles.is_empty() && m.vertices.is_empty());
    }

    #[test]
    fn empty_mesh_produces_valid_header_only_stl() {
        // 問260: gltf.rs/html.rs は空メッシュのテストを持つが stl.rs は持っていなかった
        // (フォーマット横断の一貫性欠落)。空メッシュでもパニックせず、80バイトヘッダ +
        // 4バイト三角形数(=0) の 84 バイトちょうどを返すことを固定する。
        use crate::extract::Mesh;
        let empty = Mesh::default();
        let bytes = encode_binary(&empty);
        assert_eq!(
            bytes.len(),
            84,
            "empty mesh STL must be exactly header+count (84 bytes)"
        );
        let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap());
        assert_eq!(count, 0, "empty mesh triangle count field must be 0");
        // ヘッダタグが含まれる (壊れたバイト列でない)。
        assert!(
            bytes
                .windows(b"kado binary stl".len())
                .any(|w| w == b"kado binary stl"),
            "empty mesh STL must still carry the header tag"
        );
    }
}
