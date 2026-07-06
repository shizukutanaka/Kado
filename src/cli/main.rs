//! Kado CLI。
//!
//! コマンド:
//!   version    バージョン表示
//!   selftest   最小 SDF 評価の動作確認
//!   export     [scene.json] <out.stl|.glb|.3mf|.html>  メッシュ出力 (拡張子で形式選択)
//!   screenshot [scene.json] <out.png> [view]  PNG スクリーンショット出力
//!   run        <scene.json> [resolution]  メッシュ統計表示
//!   check      <scene.json> [min_wall_mm] [max_overhang_deg] [resolution]  DFM 検証
//!   mcp        MCP サーバー (stdio) 起動

use kado::core::{Sdf, Vec3};
use kado::extract::polygonize;
use kado::io::ExportFormat;
use kado::mcp::server::run_stdio;
use kado::render::{draw_axes, render, Camera};
use kado::script::eval_any;
use kado::verify::{validate, validate_with_field};

fn demo_model() -> Sdf {
    // 問78: smooth_union で球と直方体をブレンドし、SDF の有機的な特長をデモとして示す。
    Sdf::sphere(1.0).smooth_union(Sdf::cuboid(Vec3::splat(0.8)), 0.2)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("version");

    match cmd {
        "version" | "--version" | "-V" => {
            println!("kado {}", env!("CARGO_PKG_VERSION"));
        }
        // 問275: help は正常系 (stdout・exit 0)。unknown command のエラー経路
        // (stderr・exit 2) と区別する — 市販 CLI の基本作法。
        "help" | "--help" | "-h" => {
            println!("{}", usage_text());
        }
        "selftest" => {
            let d = demo_model().eval(Vec3::ZERO);
            println!("selftest ok: f(origin) = {d}");
        }
        "export" => {
            // export [scene.json] <out.stl>
            // arg2 が .json で終わる → scene file; arg3 が出力パス。
            // arg2 がなければデモモデルを kado-demo.stl に出力。
            let (sdf, out) = if args.get(2).map(|s| s.ends_with(".json")).unwrap_or(false) {
                let src = load_scene_file(args.get(2).unwrap());
                let sdf = parse_scene(&src);
                let out = args.get(3).map(String::as_str).unwrap_or("kado-export.stl");
                (sdf, out.to_string())
            } else {
                let out = args.get(2).map(String::as_str).unwrap_or("kado-demo.stl");
                (demo_model(), out.to_string())
            };
            let (lo, hi) = sdf.sampling_box();
            let mesh = polygonize(&sdf, lo, hi, 64);
            // 問48: screenshot と同様に空メッシュを検出して早期終了。
            // 空 STL を無音で書き出すのではなく、境界拡大のヒントを出す。
            if mesh.triangles.is_empty() {
                eprintln!("mesh is empty — bounding box may not contain the shape");
                std::process::exit(1);
            }
            // 拡張子で形式を選択 (問124: MCP と共有する単一の真実源 io::ExportFormat)。
            let path = std::path::Path::new(&out);
            let format = ExportFormat::from_path(&out);
            let write_res = format.write(&mesh, path);
            match write_res {
                Ok(()) => println!(
                    "exported {} ({} triangles, manifold={})",
                    out,
                    mesh.triangles.len(),
                    mesh.is_edge_manifold()
                ),
                Err(e) => {
                    eprintln!("export failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        "screenshot" => {
            // screenshot [scene.json] <out.png> [view]
            // arg2 が .json で終わる → scene file; arg3 が出力パス; arg4 がビュー。
            let (sdf, out, view) = if args.get(2).map(|s| s.ends_with(".json")).unwrap_or(false) {
                let src = load_scene_file(args.get(2).unwrap());
                let sdf = parse_scene(&src);
                let out = args
                    .get(3)
                    .map(String::as_str)
                    .unwrap_or("kado-screenshot.png");
                let view = args.get(4).map(String::as_str).unwrap_or("iso");
                (sdf, out.to_string(), view.to_string())
            } else {
                let out = args
                    .get(2)
                    .map(String::as_str)
                    .unwrap_or("kado-screenshot.png");
                let view = args.get(3).map(String::as_str).unwrap_or("iso");
                (demo_model(), out.to_string(), view.to_string())
            };
            let (lo_b, hi_b) = sdf.sampling_box();
            let mesh = polygonize(&sdf, lo_b, hi_b, 48);
            if mesh.triangles.is_empty() {
                eprintln!("mesh is empty");
                std::process::exit(1);
            }
            let (lo, hi) = mesh.bounds().unwrap();
            let presets = Camera::presets(lo, hi);
            // 問71: 未知ビュー名は明示エラー (サイレントフォールバックしない)。
            let cam = match presets.iter().find(|(n, _)| *n == view.as_str()) {
                Some((_, c)) => c.clone(),
                None => {
                    let valid: Vec<&str> = presets.iter().map(|(n, _)| *n).collect();
                    eprintln!("unknown view '{view}'; valid: {}", valid.join(", "));
                    std::process::exit(2);
                }
            };
            // 2× スーパーサンプルしてアンチエイリアス (問56)。
            let mut img = render(&mesh, &cam, 1024, 1024).downsample(2);
            // 向きの基準として座標軸グノモンを重ねる (問66)。
            let center = (lo + hi) * 0.5;
            draw_axes(&mut img, &cam, center, (hi - lo).length() * 0.35);
            match img.write_png(std::path::Path::new(&out)) {
                Ok(()) => println!(
                    "screenshot {} ({} triangles, view={})",
                    out,
                    mesh.triangles.len(),
                    view
                ),
                Err(e) => {
                    eprintln!("screenshot failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        "run" => {
            // run <scene.json> [resolution]
            let path = args.get(2).map(String::as_str).unwrap_or_else(|| {
                eprintln!("usage: kado run <scene.json> [resolution]");
                std::process::exit(2);
            });
            let res: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(48);
            let res = res.clamp(1, 256);
            let sdf = parse_scene(&load_scene_file(path));
            let (lo, hi) = sdf.sampling_box();
            let mesh = polygonize(&sdf, lo, hi, res);
            let report = validate(&mesh, 0.0, 0.0);
            println!("{}", report.summary());
        }
        "check" => {
            // check <scene.json> [min_wall_mm] [max_overhang_deg] [resolution]
            let path = args.get(2).map(String::as_str).unwrap_or_else(|| {
                eprintln!(
                    "usage: kado check <scene.json> [min_wall_mm] [max_overhang_deg] [resolution]"
                );
                std::process::exit(2);
            });
            let min_wall =
                parse_finite_arg(args.get(3), "min_wall_mm", 0.5).unwrap_or_else(|e| fail(e));
            let max_overhang =
                parse_finite_arg(args.get(4), "max_overhang_deg", 45.0).unwrap_or_else(|e| fail(e));
            let res: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(48);
            let res = res.clamp(1, 256);
            // 問47: run と同じヘルパを使い、ファイル読み込み・評価のエラー処理を一本化する。
            let sdf = parse_scene(&load_scene_file(path));
            let (lo, hi) = sdf.sampling_box();
            let mesh = polygonize(&sdf, lo, hi, res);
            // SDF を渡し局所薄肉の内向きレイ探針を有効化する (問58)。ビルド方向 +Z (問68)。
            let report = validate_with_field(
                &mesh,
                Some(&sdf),
                min_wall,
                max_overhang,
                Vec3::new(0.0, 0.0, 1.0),
            );
            let status = if report.is_ok() { "PASS" } else { "FAIL" };
            println!("[{status}] {}", report.summary());
            for issue in &report.issues {
                println!("  [{:?}] {} — {}", issue.severity, issue.code, issue.cause);
                for hint in &issue.fix_hints {
                    println!("    hint: {hint}");
                }
            }
            if !report.is_ok() {
                std::process::exit(1);
            }
        }
        "mcp" => {
            // MCP サーバーモード (stdin/stdout・返らない)。
            run_stdio();
        }
        other => {
            eprintln!("unknown command: {other}");
            eprintln!("{}", usage_text());
            std::process::exit(2);
        }
    }
}

/// 全コマンドの使い方一覧 (問275)。`help` (正常系・stdout) と unknown command
/// (エラー・stderr) の両方が同じ文面を使い、一覧の二重管理を避ける。
/// ファイル冒頭のモジュールコメント (問272) と同内容を保つこと。
fn usage_text() -> &'static str {
    // 生の複数行リテラル: `\` 行継続は行頭空白を剥がしインデントが崩れるため使わない。
    "usage: kado <command> [args]

commands:
  version                                    show version
  selftest                                   minimal SDF evaluation check
  export     [scene.json] <out.stl|.glb|.3mf|.html>
                                             export mesh (format by extension)
  screenshot [scene.json] <out.png> [view]   render PNG screenshot
  run        <scene.json> [resolution]       show mesh statistics
  check      <scene.json> [min_wall_mm] [max_overhang_deg] [resolution]
                                             DFM validation
  mcp                                        start MCP server (stdio)
  help                                       show this message

running with no command prints the version."
}

fn load_scene_file(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            std::process::exit(1);
        }
    }
}

fn parse_scene(src: &str) -> Sdf {
    // JSON / テキスト DSL を自動判別 (問59)。
    match eval_any(src) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("script error: {e}");
            std::process::exit(1);
        }
    }
}

/// 数値引数を解析する。省略時は既定値、パース失敗または非有限 (NaN/Inf) は
/// エラーを返す (問262)。
///
/// MCP 経由 (mcp/tools.rs) は JSON 自体が NaN/Infinity を表現できないため、
/// `min_wall_mm`/`max_overhang_deg` の非有限値は構造的に届かない。しかし CLI は
/// コマンドライン文字列を `str::parse::<f64>()` で読むため "nan"/"inf"/"-inf" が
/// 有効な f64 として通ってしまう。これを `unwrap_or(default)` で握りつぶすと、
/// タイプミス (例: `kado check scene.json abc`) が既定値実行として静かに進行し、
/// 気付かれない。省略 (引数なし) と誤入力 (パース失敗/非有限) を区別し、後者のみ
/// エラーにする。
fn parse_finite_arg(raw: Option<&String>, name: &str, default: f64) -> Result<f64, String> {
    match raw {
        None => Ok(default),
        Some(s) => match s.parse::<f64>() {
            Ok(v) if v.is_finite() => Ok(v),
            Ok(v) => Err(format!("{name} must be finite, got {v}")),
            Err(_) => Err(format!("{name} must be a number, got {s:?}")),
        },
    }
}

fn fail(msg: String) -> ! {
    eprintln!("{msg}");
    std::process::exit(2);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_finite_arg_uses_default_when_omitted() {
        // 問262: 引数省略 (None) は誤入力ではないので既定値。
        assert_eq!(parse_finite_arg(None, "x", 0.5), Ok(0.5));
    }

    #[test]
    fn parse_finite_arg_accepts_valid_finite_numbers() {
        let s = "1.25".to_string();
        assert_eq!(parse_finite_arg(Some(&s), "x", 0.5), Ok(1.25));
        let neg = "-3".to_string();
        assert_eq!(parse_finite_arg(Some(&neg), "x", 0.5), Ok(-3.0));
    }

    #[test]
    fn parse_finite_arg_rejects_nan_and_infinity_strings() {
        // 問262: str::parse::<f64>() は "nan"/"inf"/"-inf" を有効な f64 として
        // 受理してしまう (Rust FromStr の仕様)。CLI はこれを既定値へ静かに
        // フォールバックさせず、明示エラーにしなければならない。
        for bad in ["nan", "NaN", "inf", "-inf", "infinity"] {
            let s = bad.to_string();
            let r = parse_finite_arg(Some(&s), "min_wall_mm", 0.5);
            assert!(
                r.is_err(),
                "{bad:?} must be rejected as non-finite, got {r:?}"
            );
        }
    }

    #[test]
    fn parse_finite_arg_rejects_unparseable_strings() {
        // 問262: "abc" のような非数値文字列は以前は既定値へ静かにフォールバック
        // していた。誤入力として明示エラーにする。
        let s = "abc".to_string();
        let r = parse_finite_arg(Some(&s), "min_wall_mm", 0.5);
        assert!(r.is_err(), "non-numeric string must be rejected, got {r:?}");
    }

    #[test]
    fn usage_text_lists_every_command() {
        // 問275: help が表示する一覧に全コマンドが含まれることを固定する
        // (問272 型のドリフト防止: main() の match アームへコマンドを追加したら
        // usage_text も更新しないとこのテストが落ちる)。
        let text = usage_text();
        for cmd in [
            "version",
            "selftest",
            "export",
            "screenshot",
            "run",
            "check",
            "mcp",
            "help",
        ] {
            assert!(
                text.contains(cmd),
                "usage_text must list command '{cmd}' (問275)"
            );
        }
        // 引数仕様も要点を含む (問272 で直した [resolution] を含む)。
        assert!(
            text.contains("[resolution]"),
            "usage_text must document the optional [resolution] arg"
        );
    }
}
