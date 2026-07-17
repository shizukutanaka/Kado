//! 出力フォーマット (Plan §3: STL / 3MF / GLB / HTMLビューア)。
//!
//! 製造の最小共通項 **binary STL**、インデックス付き・閲覧容易な
//! **GLB (glTF 2.0 binary)**、現代的プリント標準 **3MF** (単位付き OPC/ZIP)、
//! オフライン閲覧用の **自己完結 HTML ビューア** (WebGL2) を実装する
//! (facetted STEP は問7で BACKLOG 降格)。

pub mod gltf;
pub mod html;
pub mod stl;
pub mod threemf;
pub mod zip;

use crate::extract::Mesh;
use std::path::Path;

/// 出力ファイル形式。**拡張子から決定する単一の真実源** (問124)。
///
/// 以前は CLI (`cli/main.rs`) と MCP (`tools::tool_export`) がそれぞれ独立に
/// 「.glb→GLB / .3mf→3MF / .html→HTML / その他→STL」の if-else を持っていた。
/// 形式を追加・変更するとき片方だけ直すと両者がサイレントに食い違う (問120 と同じ
/// 重複リスク)。ここに一元化し、両入口が同じ判定・同じライタを使うことを構造的に保証する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    /// binary STL (製造の最小共通項・既定フォールバック)。
    Stl,
    /// glTF 2.0 binary。
    Glb,
    /// 3MF (単位付き OPC/ZIP)。
    ThreeMf,
    /// 自己完結 HTML ビューア。
    Html,
}

impl ExportFormat {
    /// パス拡張子から形式を決定する。未知拡張子は STL にフォールバック (問54/55/57)。
    /// 大文字小文字は無視する。
    pub fn from_path(path: &str) -> ExportFormat {
        let lower = path.to_lowercase();
        if lower.ends_with(".glb") {
            ExportFormat::Glb
        } else if lower.ends_with(".3mf") {
            ExportFormat::ThreeMf
        } else if lower.ends_with(".html") || lower.ends_with(".htm") {
            ExportFormat::Html
        } else {
            ExportFormat::Stl
        }
    }

    /// 応答・ログ用の短いラベル。
    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Stl => "STL",
            ExportFormat::Glb => "GLB",
            ExportFormat::ThreeMf => "3MF",
            ExportFormat::Html => "HTML",
        }
    }

    /// メッシュを所定の形式でファイルへ書き出す。
    pub fn write(self, mesh: &Mesh, path: &Path) -> std::io::Result<()> {
        match self {
            ExportFormat::Stl => stl::write_binary(mesh, path),
            ExportFormat::Glb => gltf::write_glb(mesh, path),
            ExportFormat::ThreeMf => threemf::write_3mf(mesh, path),
            ExportFormat::Html => html::write_html(mesh, path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_path_dispatches_by_extension() {
        // 問124: 拡張子→形式の判定を固定する。CLI と MCP が共有する単一の真実源。
        assert_eq!(ExportFormat::from_path("model.glb"), ExportFormat::Glb);
        assert_eq!(ExportFormat::from_path("model.3mf"), ExportFormat::ThreeMf);
        assert_eq!(ExportFormat::from_path("model.html"), ExportFormat::Html);
        assert_eq!(ExportFormat::from_path("model.htm"), ExportFormat::Html);
        assert_eq!(ExportFormat::from_path("model.stl"), ExportFormat::Stl);
        // 未知拡張子・拡張子なしは STL フォールバック。
        assert_eq!(ExportFormat::from_path("model.xyz"), ExportFormat::Stl);
        assert_eq!(ExportFormat::from_path("model"), ExportFormat::Stl);
    }

    #[test]
    fn from_path_is_case_insensitive() {
        // 大文字拡張子でも同じ形式 (AI が ".GLB" を渡しても STL に落ちない)。
        assert_eq!(ExportFormat::from_path("M.GLB"), ExportFormat::Glb);
        assert_eq!(ExportFormat::from_path("M.3MF"), ExportFormat::ThreeMf);
        assert_eq!(ExportFormat::from_path("M.HTML"), ExportFormat::Html);
        assert_eq!(ExportFormat::from_path("M.Stl"), ExportFormat::Stl);
    }

    #[test]
    fn label_matches_format() {
        assert_eq!(ExportFormat::Stl.label(), "STL");
        assert_eq!(ExportFormat::Glb.label(), "GLB");
        assert_eq!(ExportFormat::ThreeMf.label(), "3MF");
        assert_eq!(ExportFormat::Html.label(), "HTML");
    }

    #[test]
    fn write_produces_format_specific_bytes() {
        // 各形式が固有のマジック/署名で始まることを確認 (write が正しいライタへ振り分ける)。
        use crate::core::{Sdf, Vec3};
        use crate::extract::polygonize;
        let mesh = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 8);
        let dir = std::env::temp_dir();

        // GLB: "glTF" マジック。
        let glb = dir.join("kado_fmt_test.glb");
        ExportFormat::Glb.write(&mesh, &glb).unwrap();
        let glb_bytes = std::fs::read(&glb).unwrap();
        assert_eq!(&glb_bytes[0..4], b"glTF", "GLB must start with glTF magic");
        let _ = std::fs::remove_file(&glb);

        // 3MF: ZIP ローカルヘッダ署名 "PK\x03\x04"。
        let mf = dir.join("kado_fmt_test.3mf");
        ExportFormat::ThreeMf.write(&mesh, &mf).unwrap();
        let mf_bytes = std::fs::read(&mf).unwrap();
        assert_eq!(
            &mf_bytes[0..4],
            &[0x50, 0x4B, 0x03, 0x04],
            "3MF must be a ZIP"
        );
        let _ = std::fs::remove_file(&mf);

        // HTML: "<!DOCTYPE html>" で始まる。
        let html = dir.join("kado_fmt_test.html");
        ExportFormat::Html.write(&mesh, &html).unwrap();
        let html_bytes = std::fs::read(&html).unwrap();
        assert!(
            html_bytes.starts_with(b"<!DOCTYPE html>"),
            "HTML must start with doctype"
        );
        let _ = std::fs::remove_file(&html);

        // STL: 80 バイトヘッダ "kado binary stl"。
        let stl = dir.join("kado_fmt_test.stl");
        ExportFormat::Stl.write(&mesh, &stl).unwrap();
        let stl_bytes = std::fs::read(&stl).unwrap();
        assert!(
            stl_bytes.starts_with(b"kado binary stl"),
            "STL must have kado header"
        );
        let _ = std::fs::remove_file(&stl);
    }

    #[test]
    fn all_export_formats_re_encode_byte_identically() {
        // 問203 (SPEC §6): "STL/GLB/PNG は同一メッシュからバイト同一"。
        // 各形式は個別に determinism テストを持つが、同一メッシュから 4 形式すべてが
        // バイト同一に再エンコードされることを 1 つのテストで横断的に固定する。
        // 1 形式だけ決定性が壊れる回帰を確実に検出する回帰防壁。
        use crate::core::{Sdf, Vec3};
        use crate::extract::polygonize;
        let mesh = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 12);

        assert_eq!(
            stl::encode_binary(&mesh),
            stl::encode_binary(&mesh),
            "STL must re-encode identically"
        );
        assert_eq!(
            gltf::encode_glb(&mesh),
            gltf::encode_glb(&mesh),
            "GLB must re-encode identically"
        );
        assert_eq!(
            threemf::encode_3mf(&mesh),
            threemf::encode_3mf(&mesh),
            "3MF must re-encode identically"
        );
        assert_eq!(
            html::encode_html(&mesh),
            html::encode_html(&mesh),
            "HTML must re-encode identically"
        );
        // メッシュダイジェスト (観測可能な決定性プロキシ) も安定。
        assert_eq!(
            mesh.digest(),
            mesh.digest(),
            "mesh digest must be deterministic"
        );
    }
}
