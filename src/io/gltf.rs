//! glTF 2.0 binary (GLB) 書き出し。
//!
//! 決定的 (問5): リトルエンディアン・頂点/三角形はメッシュ順・固定 JSON 構造。
//! 外部依存ゼロ (std のみ, ADR-003 / 問4)。
//!
//! STL と違い**インデックス付き**ジオメトリと境界 (accessor min/max) を持ち、
//! ブラウザ・Windows 3D ビューア・Blender 等で直接閲覧できる。圧縮は使わない。

use crate::extract::Mesh;
use crate::mcp::json;

// GLB マジック・チャンク種別 (u32 LE)。
const MAGIC_GLTF: u32 = 0x4654_6C67; // "glTF"
const CHUNK_JSON: u32 = 0x4E4F_534A; // "JSON"
const CHUNK_BIN: u32 = 0x004E_4942; // "BIN\0"

// glTF 定数。
const COMPONENT_FLOAT: f64 = 5126.0; // f32
const COMPONENT_UINT: f64 = 5125.0; // u32
const MODE_TRIANGLES: f64 = 4.0;
const TARGET_ARRAY_BUFFER: f64 = 34962.0; // 頂点属性
const TARGET_ELEMENT_ARRAY: f64 = 34963.0; // インデックス

/// メッシュを GLB バイト列にエンコードする。
pub fn encode_glb(mesh: &Mesh) -> Vec<u8> {
    // ── BIN バッファ: POSITION(f32×3) … その後 indices(u32) ──
    let mut bin = Vec::with_capacity(mesh.vertices.len() * 12 + mesh.triangles.len() * 12);
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for v in &mesh.vertices {
        let f = [v.x as f32, v.y as f32, v.z as f32];
        for k in 0..3 {
            if f[k] < min[k] {
                min[k] = f[k];
            }
            if f[k] > max[k] {
                max[k] = f[k];
            }
            bin.extend_from_slice(&f[k].to_le_bytes());
        }
    }
    let pos_len = bin.len();
    for t in &mesh.triangles {
        for &i in t {
            bin.extend_from_slice(&i.to_le_bytes());
        }
    }
    let idx_len = bin.len() - pos_len;
    // 空メッシュでは min/max が無限大のままなので 0 に正規化する。
    if mesh.vertices.is_empty() {
        min = [0.0; 3];
        max = [0.0; 3];
    }

    let vcount = mesh.vertices.len() as f64;
    let icount = (mesh.triangles.len() * 3) as f64;

    // POSITION accessor は min/max が必須 (glTF 仕様)。
    let minmax = |a: [f32; 3]| {
        json::arr([
            json::n(a[0] as f64),
            json::n(a[1] as f64),
            json::n(a[2] as f64),
        ])
    };

    let doc = json::obj([
        (
            "asset",
            json::obj([("version", json::s("2.0")), ("generator", json::s("kado"))]),
        ),
        ("scene", json::n(0.0)),
        (
            "scenes",
            json::arr([json::obj([("nodes", json::arr([json::n(0.0)]))])]),
        ),
        ("nodes", json::arr([json::obj([("mesh", json::n(0.0))])])),
        (
            "meshes",
            json::arr([json::obj([(
                "primitives",
                json::arr([json::obj([
                    ("attributes", json::obj([("POSITION", json::n(0.0))])),
                    ("indices", json::n(1.0)),
                    ("mode", json::n(MODE_TRIANGLES)),
                ])]),
            )])]),
        ),
        (
            "buffers",
            json::arr([json::obj([("byteLength", json::n(bin.len() as f64))])]),
        ),
        (
            "bufferViews",
            json::arr([
                json::obj([
                    ("buffer", json::n(0.0)),
                    ("byteOffset", json::n(0.0)),
                    ("byteLength", json::n(pos_len as f64)),
                    ("target", json::n(TARGET_ARRAY_BUFFER)),
                ]),
                json::obj([
                    ("buffer", json::n(0.0)),
                    ("byteOffset", json::n(pos_len as f64)),
                    ("byteLength", json::n(idx_len as f64)),
                    ("target", json::n(TARGET_ELEMENT_ARRAY)),
                ]),
            ]),
        ),
        (
            "accessors",
            json::arr([
                json::obj([
                    ("bufferView", json::n(0.0)),
                    ("componentType", json::n(COMPONENT_FLOAT)),
                    ("count", json::n(vcount)),
                    ("type", json::s("VEC3")),
                    ("min", minmax(min)),
                    ("max", minmax(max)),
                ]),
                json::obj([
                    ("bufferView", json::n(1.0)),
                    ("componentType", json::n(COMPONENT_UINT)),
                    ("count", json::n(icount)),
                    ("type", json::s("SCALAR")),
                ]),
            ]),
        ),
    ]);

    // JSON チャンクは 4 バイト境界へスペース (0x20) パディング。
    let mut json_bytes = doc.to_string().into_bytes();
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }
    // BIN チャンクは 4 バイト境界へゼロパディング (f32×3/u32 なので通常は整列済み)。
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }

    let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    // 12 バイトヘッダ。
    out.extend_from_slice(&MAGIC_GLTF.to_le_bytes());
    out.extend_from_slice(&2u32.to_le_bytes()); // version 2
    out.extend_from_slice(&(total as u32).to_le_bytes());
    // JSON チャンク。
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_JSON.to_le_bytes());
    out.extend_from_slice(&json_bytes);
    // BIN チャンク。
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_BIN.to_le_bytes());
    out.extend_from_slice(&bin);
    out
}

/// メッシュを GLB ファイルに書き出す。
pub fn write_glb(mesh: &Mesh, path: &std::path::Path) -> std::io::Result<()> {
    std::fs::write(path, encode_glb(mesh))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Sdf, Vec3};
    use crate::extract::polygonize;
    use crate::mcp::json::parse;

    fn sphere_mesh() -> Mesh {
        polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 16)
    }

    #[test]
    fn glb_header_is_valid() {
        let bytes = encode_glb(&sphere_mesh());
        assert_eq!(
            u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            MAGIC_GLTF
        );
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 2);
        let total = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        assert_eq!(
            total,
            bytes.len(),
            "header total length must equal byte length"
        );
    }

    #[test]
    fn glb_chunks_are_consistent_and_aligned() {
        let bytes = encode_glb(&sphere_mesh());
        // JSON チャンク。
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        assert_eq!(
            u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            CHUNK_JSON
        );
        assert_eq!(json_len % 4, 0, "JSON chunk must be 4-byte aligned");
        let json_end = 20 + json_len;
        // BIN チャンク。
        let bin_len =
            u32::from_le_bytes(bytes[json_end..json_end + 4].try_into().unwrap()) as usize;
        assert_eq!(
            u32::from_le_bytes(bytes[json_end + 4..json_end + 8].try_into().unwrap()),
            CHUNK_BIN
        );
        assert_eq!(bin_len % 4, 0, "BIN chunk must be 4-byte aligned");
        assert_eq!(json_end + 8 + bin_len, bytes.len());
    }

    #[test]
    fn glb_json_describes_mesh_accurately() {
        let mesh = sphere_mesh();
        let bytes = encode_glb(&mesh);
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let json_str = std::str::from_utf8(&bytes[20..20 + json_len])
            .unwrap()
            .trim_end();
        let doc = parse(json_str).expect("GLB JSON chunk must be valid JSON");
        // POSITION accessor の count が頂点数と一致。
        let accessors = doc.get("accessors").and_then(|a| a.as_array()).unwrap();
        let pos_count = accessors[0].get("count").and_then(|c| c.as_f64()).unwrap();
        assert_eq!(pos_count as usize, mesh.vertices.len());
        // indices accessor の count が三角形数×3 と一致。
        let idx_count = accessors[1].get("count").and_then(|c| c.as_f64()).unwrap();
        assert_eq!(idx_count as usize, mesh.triangles.len() * 3);
    }

    #[test]
    fn glb_accessor_bufferview_indices_are_correctly_wired() {
        // 問221: glb_json_describes_mesh_accurately は count のみ確認。
        // accessor[0]→bufferView 0 (POSITION)、accessor[1]→bufferView 1 (INDEX) の
        // 参照配線と bufferView が厳密に 2 個であることが未検証だった。
        // 配線が入れ替わると GLB ビューアが頂点/索引を取り違える。
        let bytes = encode_glb(&sphere_mesh());
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let doc = parse(
            std::str::from_utf8(&bytes[20..20 + json_len])
                .unwrap()
                .trim_end(),
        )
        .unwrap();
        let accessors = doc.get("accessors").and_then(|a| a.as_array()).unwrap();
        let views = doc.get("bufferViews").and_then(|v| v.as_array()).unwrap();
        assert_eq!(
            views.len(),
            2,
            "must have exactly 2 bufferViews (POSITION, INDEX)"
        );
        assert_eq!(
            accessors[0].get("bufferView").and_then(|x| x.as_f64()),
            Some(0.0),
            "POSITION accessor must reference bufferView 0"
        );
        assert_eq!(
            accessors[1].get("bufferView").and_then(|x| x.as_f64()),
            Some(1.0),
            "INDEX accessor must reference bufferView 1"
        );
        // 各 bufferView は buffer 0 を参照する。
        for (k, view) in views.iter().enumerate() {
            assert_eq!(
                view.get("buffer").and_then(|x| x.as_f64()),
                Some(0.0),
                "bufferView {k} must reference buffer 0"
            );
        }
    }

    #[test]
    fn glb_buffer_byte_lengths_are_internally_consistent() {
        // 問222: bufferView の byteLength・byteOffset・buffers[0].byteLength・BIN チャンク
        // ヘッダの整合 (pos_len + idx_len == total) が未検証だった。
        // view0.byteOffset=0, view1.byteOffset=pos_len, 合計=buffer 全長を確認する。
        let bytes = encode_glb(&sphere_mesh());
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let json_end = 20 + json_len;
        let bin_len_header =
            u32::from_le_bytes(bytes[json_end..json_end + 4].try_into().unwrap()) as usize;
        let doc = parse(
            std::str::from_utf8(&bytes[20..json_end])
                .unwrap()
                .trim_end(),
        )
        .unwrap();

        let buffers = doc.get("buffers").and_then(|a| a.as_array()).unwrap();
        let declared_total = buffers[0]
            .get("byteLength")
            .and_then(|x| x.as_f64())
            .unwrap() as usize;
        let views = doc.get("bufferViews").and_then(|v| v.as_array()).unwrap();
        let v0_off = views[0].get("byteOffset").and_then(|x| x.as_f64()).unwrap() as usize;
        let v0_len = views[0].get("byteLength").and_then(|x| x.as_f64()).unwrap() as usize;
        let v1_off = views[1].get("byteOffset").and_then(|x| x.as_f64()).unwrap() as usize;
        let v1_len = views[1].get("byteLength").and_then(|x| x.as_f64()).unwrap() as usize;

        // view0 は先頭、view1 は view0 の直後 (連続配置)。
        assert_eq!(v0_off, 0, "POSITION bufferView must start at offset 0");
        assert_eq!(
            v1_off, v0_len,
            "INDEX bufferView must start right after POSITION"
        );
        // bufferView の合計 == buffer 宣言長 (パディング前は厳密一致)。
        assert_eq!(
            v0_len + v1_len,
            declared_total,
            "bufferView byteLengths must sum to buffer byteLength"
        );
        // BIN チャンクヘッダ長は宣言長以上 (4 バイト境界パディングを許容)。
        assert!(
            bin_len_header >= declared_total,
            "BIN chunk must hold the full buffer (+padding)"
        );
        assert!(
            bin_len_header - declared_total < 4,
            "BIN padding must be < 4 bytes"
        );
    }

    #[test]
    fn glb_encoding_is_deterministic() {
        let m = sphere_mesh();
        assert_eq!(encode_glb(&m), encode_glb(&m));
    }

    #[test]
    fn glb_accessor_min_max_correct_for_single_triangle() {
        // 問202: glb_json_describes_mesh_accurately は多頂点 sphere のみ確認。
        // 単一三角形 (3 頂点) で outlier 頂点を含む場合に
        // POSITION accessor の min/max が正しく計算されることを確認する。
        // encode_glb の min/max ループ (lines 30-41) が頂点 1 枚でも正常に動くことを固定。
        let mesh = crate::extract::Mesh::from_soup(&[[
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 3.0), // outlier
        ]]);
        // from_soup は重複除去するので頂点数は 3 になるはず。
        assert_eq!(
            mesh.vertices.len(),
            3,
            "single triangle must have 3 vertices"
        );
        let bytes = encode_glb(&mesh);
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let json_str = std::str::from_utf8(&bytes[20..20 + json_len])
            .unwrap()
            .trim_end();
        let doc = parse(json_str).expect("single-triangle GLB must be valid JSON");
        let accessors = doc.get("accessors").and_then(|a| a.as_array()).unwrap();
        let pos_min = accessors[0].get("min").and_then(|a| a.as_array()).unwrap();
        let pos_max = accessors[0].get("max").and_then(|a| a.as_array()).unwrap();
        // min は全成分 0、max は outlier 頂点に引っ張られる。
        assert_eq!(pos_min[0].as_f64(), Some(0.0), "min.x must be 0");
        assert_eq!(pos_min[1].as_f64(), Some(0.0), "min.y must be 0");
        assert_eq!(pos_min[2].as_f64(), Some(0.0), "min.z must be 0");
        assert_eq!(pos_max[0].as_f64(), Some(1.0), "max.x must be 1");
        assert_eq!(pos_max[1].as_f64(), Some(2.0), "max.y must be 2");
        assert_eq!(pos_max[2].as_f64(), Some(3.0), "max.z must be 3");
    }

    #[test]
    fn empty_mesh_produces_valid_parseable_glb() {
        // 問140: 空メッシュの encode_glb がパニックせず、有効な GLB / JSON を返す。
        // encode_glb の empty guard (lines 50-53) は min/max を [0,0,0] に正規化するが
        // この経路のテストがなかった。
        let empty = Mesh::default();
        let bytes = encode_glb(&empty);
        // GLB magic/version ヘッダ。
        assert_eq!(
            u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            MAGIC_GLTF
        );
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 2);
        // JSON チャンクが valid (panic せず parse できる)。
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let json_str = std::str::from_utf8(&bytes[20..20 + json_len])
            .unwrap()
            .trim_end();
        let doc = parse(json_str).expect("empty mesh GLB JSON chunk must be valid JSON");
        // 両 accessor の count が 0。
        let accessors = doc
            .get("accessors")
            .and_then(|a| a.as_array())
            .expect("must have accessors");
        assert_eq!(
            accessors.len(),
            2,
            "must have exactly 2 accessors (POSITION, INDEX)"
        );
        for (k, acc) in accessors.iter().enumerate() {
            let count = acc.get("count").and_then(|c| c.as_f64()).unwrap_or(-1.0) as i64;
            assert_eq!(count, 0, "accessor {k} count must be 0 for empty mesh");
        }
    }
}
