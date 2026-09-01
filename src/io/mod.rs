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
    /// 拡張子 → 形式の対応表 (単一の真実源)。
    ///
    /// エラーメッセージの候補一覧もここから生成するので、形式を足したときに
    /// 「対応は増えたがメッセージは古いまま」が起こらない (問319 と同じ規律)。
    const BY_EXTENSION: &'static [(&'static str, ExportFormat)] = &[
        ("stl", ExportFormat::Stl),
        ("glb", ExportFormat::Glb),
        ("3mf", ExportFormat::ThreeMf),
        ("html", ExportFormat::Html),
        ("htm", ExportFormat::Html),
    ];

    /// 受理する拡張子の一覧 (エラーメッセージ用)。
    pub fn supported_extensions() -> String {
        Self::BY_EXTENSION
            .iter()
            .map(|(e, _)| format!(".{e}"))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// パス拡張子から形式を決定する。大文字小文字は無視する。
    ///
    /// 問322: 以前は**未知の拡張子を黙って STL にフォールバック**していた。
    /// その結果 `export model.obj` は「中身は binary STL・名前は .obj」という
    /// **拡張子が内容について嘘をつくファイル**を無言で作り、利用者は Kado が
    /// OBJ を書けたものと信じてしまう (実測: `.xyz` 出力は `.stl` 出力と
    /// バイト単位で同一だった)。問318 で MCP 引数に適用した規則
    /// ——**省略は既定・指定されたが解釈できなければエラー**——をここにも適用する。
    ///
    /// 拡張子が**無い**場合は STL 既定のままにする。`kado export out` は
    /// 何とも矛盾しないので、拒否する理由が無い (拒否は入力が誤っている場合だけ)。
    pub fn from_path(path: &str) -> Result<ExportFormat, String> {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());
        match ext {
            None => Ok(ExportFormat::Stl),
            Some(e) if e.is_empty() => Ok(ExportFormat::Stl),
            Some(e) => Self::BY_EXTENSION
                .iter()
                .find(|(known, _)| *known == e)
                .map(|(_, f)| *f)
                .ok_or_else(|| {
                    format!(
                        "unsupported output format '.{e}' in '{path}'; supported: {}. \
                         Writing STL bytes under a '.{e}' name would make the file \
                         misrepresent its own contents.",
                        Self::supported_extensions()
                    )
                }),
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
        assert_eq!(ExportFormat::from_path("model.glb"), Ok(ExportFormat::Glb));
        assert_eq!(
            ExportFormat::from_path("model.3mf"),
            Ok(ExportFormat::ThreeMf)
        );
        assert_eq!(
            ExportFormat::from_path("model.html"),
            Ok(ExportFormat::Html)
        );
        assert_eq!(ExportFormat::from_path("model.htm"), Ok(ExportFormat::Html));
        assert_eq!(ExportFormat::from_path("model.stl"), Ok(ExportFormat::Stl));
        // 拡張子が無い場合は STL 既定。`kado export out` は何とも矛盾しないので
        // 拒否する理由が無い (問322: 拒否するのは入力が誤っている場合だけ)。
        assert_eq!(ExportFormat::from_path("model"), Ok(ExportFormat::Stl));
    }

    #[test]
    fn from_path_is_case_insensitive() {
        // 大文字拡張子でも同じ形式 (AI が ".GLB" を渡しても STL に落ちない)。
        assert_eq!(ExportFormat::from_path("M.GLB"), Ok(ExportFormat::Glb));
        assert_eq!(ExportFormat::from_path("M.3MF"), Ok(ExportFormat::ThreeMf));
        assert_eq!(ExportFormat::from_path("M.HTML"), Ok(ExportFormat::Html));
        assert_eq!(ExportFormat::from_path("M.Stl"), Ok(ExportFormat::Stl));
    }

    #[test]
    fn unknown_extension_is_rejected_not_silently_written_as_stl() {
        // 問322: 以前は未知の拡張子を黙って STL にフォールバックしていた。
        // 実測で `.xyz` 出力は `.stl` 出力とバイト単位で同一だった——つまり
        // **拡張子が内容について嘘をつくファイル**を無言で作っていた。
        // `export model.obj` を投げた利用者は Kado が OBJ を書けたと信じ、
        // 別のツールで開けない理由が分からない。問318 で MCP 引数に適用した
        // 「省略は既定・指定されたが解釈できなければエラー」をここにも適用する。
        for path in [
            "model.obj",
            "model.step",
            "model.ply",
            "out.xyz",
            "a.stl.bak",
        ] {
            let err = ExportFormat::from_path(path)
                .expect_err("an unknown extension must not silently become STL");
            assert!(
                err.contains(path),
                "error must name the offending path: {err}"
            );
            // 候補一覧は BY_EXTENSION から生成されるので、抜き取りで足りる。
            for ext in [".stl", ".glb", ".3mf", ".html"] {
                assert!(
                    err.contains(ext),
                    "error must list the supported extension '{ext}': {err}"
                );
            }
        }
    }

    #[test]
    fn supported_extensions_covers_every_writable_format() {
        // 候補一覧が BY_EXTENSION から生成されることを固定する。形式を足したときに
        // 「対応は増えたがエラーメッセージは古いまま」を防ぐ (問319 と同じ規律)。
        let listed = ExportFormat::supported_extensions();
        for f in [
            ExportFormat::Stl,
            ExportFormat::Glb,
            ExportFormat::ThreeMf,
            ExportFormat::Html,
        ] {
            let has = ExportFormat::BY_EXTENSION
                .iter()
                .any(|(e, known)| *known == f && listed.contains(&format!(".{e}")));
            assert!(has, "{} must appear in supported_extensions()", f.label());
        }
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
