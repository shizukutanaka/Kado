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
}
