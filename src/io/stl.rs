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
