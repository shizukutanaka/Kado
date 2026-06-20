//! 3MF (3D Manufacturing Format) 書き出し。
//!
//! 3MF は OPC パッケージ (= ZIP コンテナ) に XML パーツを収めた現代的な
//! 3Dプリント標準。STL と違い**単位 (mm)** と水密前提のメッシュ意味論を持ち、
//! スライサが優先的に扱える。決定的・std のみ (問4/問5)。
//!
//! パッケージ構成 (最小):
//!   [Content_Types].xml   — パーツの MIME 型宣言
//!   _rels/.rels           — ルートからモデルへの関係
//!   3D/3dmodel.model      — メッシュ本体 (vertices / triangles)

use crate::extract::Mesh;
use crate::io::zip::build_zip;
use std::fmt::Write;

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>
</Types>"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rel0" Target="/3D/3dmodel.model" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>
</Relationships>"#;

/// メッシュを 3MF パッケージ (ZIP) バイト列にエンコードする。
pub fn encode_3mf(mesh: &Mesh) -> Vec<u8> {
    let model = build_model_xml(mesh);
    build_zip(&[
        ("[Content_Types].xml", CONTENT_TYPES.as_bytes()),
        ("_rels/.rels", RELS.as_bytes()),
        ("3D/3dmodel.model", model.as_bytes()),
    ])
}

/// メッシュを 3MF ファイルに書き出す。
pub fn write_3mf(mesh: &Mesh, path: &std::path::Path) -> std::io::Result<()> {
    std::fs::write(path, encode_3mf(mesh))
}

/// 非有限座標 (NaN/±Inf) を 0.0 に正規化する。
///
/// Rust の `{}` は非有限 f64 を "NaN"/"inf" と書き出すが、これは数値テキストとして
/// 無効な XML になる (問128)。通常の SDF 抽出では非有限座標は生じないが
/// (eval.rs のパラメータ検証により)、防御的に正規化して XML の有効性を保証する。
fn finite_coord(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

/// `3D/3dmodel.model` の XML 本体を生成する。
///
/// 座標・インデックスのみを書き出すため XML エスケープが必要な文字は混入しない
/// (ユーザ文字列を XML へ入れない設計)。
fn build_model_xml(mesh: &Mesh) -> String {
    let mut s = String::with_capacity(mesh.vertices.len() * 48 + mesh.triangles.len() * 48 + 512);
    s.push_str(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <model unit=\"millimeter\" xml:lang=\"en-US\" \
         xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\">\n\
         <resources>\n\
         <object id=\"1\" type=\"model\">\n\
         <mesh>\n\
         <vertices>\n",
    );
    for v in &mesh.vertices {
        // f64 の最短往復表現 (決定的, 同一arch内)。非有限座標は 0 に正規化する (問128)。
        let _ = writeln!(
            s,
            "<vertex x=\"{}\" y=\"{}\" z=\"{}\"/>",
            finite_coord(v.x),
            finite_coord(v.y),
            finite_coord(v.z)
        );
    }
    s.push_str("</vertices>\n<triangles>\n");
    for t in &mesh.triangles {
        let _ = writeln!(s, "<triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>", t[0], t[1], t[2]);
    }
    s.push_str(
        "</triangles>\n\
         </mesh>\n\
         </object>\n\
         </resources>\n\
         <build>\n\
         <item objectid=\"1\"/>\n\
         </build>\n\
         </model>\n",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Sdf, Vec3};
    use crate::extract::polygonize;

    fn sphere_mesh() -> Mesh {
        polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 16)
    }

    #[test]
    fn threemf_is_a_zip_package() {
        let bytes = encode_3mf(&sphere_mesh());
        // ZIP ローカルヘッダ署名で始まる。
        assert_eq!(&bytes[0..4], &[0x50, 0x4B, 0x03, 0x04]);
        // 必須パーツ名を含む。
        for part in ["[Content_Types].xml", "_rels/.rels", "3D/3dmodel.model"] {
            assert!(
                bytes.windows(part.len()).any(|w| w == part.as_bytes()),
                "package must contain part {part}"
            );
        }
    }

    #[test]
    fn model_xml_counts_match_mesh() {
        let mesh = sphere_mesh();
        let xml = build_model_xml(&mesh);
        assert!(xml.contains("unit=\"millimeter\""), "must declare mm units");
        let vcount = xml.matches("<vertex ").count();
        let tcount = xml.matches("<triangle ").count();
        assert_eq!(vcount, mesh.vertices.len(), "vertex count must match");
        assert_eq!(tcount, mesh.triangles.len(), "triangle count must match");
        // 単一オブジェクトを build にひとつ配置する。
        assert!(xml.contains("<item objectid=\"1\"/>"));
    }

    #[test]
    fn threemf_is_deterministic() {
        let m = sphere_mesh();
        assert_eq!(encode_3mf(&m), encode_3mf(&m));
    }

    #[test]
    fn model_xml_never_contains_nonfinite_number_strings() {
        // 問128: build_model_xml は "NaN" や "inf" を XML に出力しない。
        // 通常抽出では発生しないが、finite_coord による防御的正規化の不変条件を固定する。
        // 文字列 "NaN"/"inf"/"Inf" が含まれると 3MF パーサがエラーになる。
        let xml = build_model_xml(&sphere_mesh());
        assert!(
            !xml.contains("NaN") && !xml.contains("inf") && !xml.contains("Inf"),
            "3MF model XML must not contain non-finite number strings: found in output"
        );
        // 非有限座標を直接持つメッシュでも XML は有効なままである。
        let bad = Mesh {
            vertices: vec![
                Vec3::new(f64::NAN, 0.0, 0.0),
                Vec3::new(f64::INFINITY, 0.0, 0.0),
                Vec3::new(0.0, f64::NEG_INFINITY, 0.0),
            ],
            triangles: vec![[0, 1, 2]],
        };
        // 退化判定 (from_soup) をバイパスして直接生成したメッシュでも XML は安全。
        let bad_xml = build_model_xml(&bad);
        assert!(
            !bad_xml.contains("NaN") && !bad_xml.contains("inf") && !bad_xml.contains("Inf"),
            "finite_coord must sanitize non-finite vertices in XML output"
        );
        // 全頂点が原点に正規化されているはず。
        assert!(
            bad_xml.contains("x=\"0\" y=\"0\" z=\"0\""),
            "non-finite coords must be replaced with 0: {bad_xml}"
        );
    }
}
