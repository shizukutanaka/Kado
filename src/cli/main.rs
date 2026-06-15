//! Kado CLI。
//!
//! コマンド:
//!   version    バージョン表示
//!   selftest   最小 SDF 評価の動作確認
//!   export     [scene.json] <out.stl|.glb|.3mf|.html>  メッシュ出力 (拡張子で形式選択)
//!   screenshot [scene.json] <out.png> [view]  PNG スクリーンショット出力
//!   run        <scene.json>  メッシュ統計表示
//!   check      <scene.json> [min_wall_mm] [max_overhang_deg]  DFM 検証
//!   mcp        MCP サーバー (stdio) 起動

use kado::core::{Sdf, Vec3};
use kado::extract::polygonize;
use kado::io::{gltf, html, stl, threemf};
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
            // 拡張子で形式を選択: .glb→GLB, .3mf→3MF, .html→HTMLビューア, 他→STL (問54/55/57)。
            let path = std::path::Path::new(&out);
            let lower = out.to_lowercase();
            let write_res = if lower.ends_with(".glb") {
                gltf::write_glb(&mesh, path)
            } else if lower.ends_with(".3mf") {
                threemf::write_3mf(&mesh, path)
            } else if lower.ends_with(".html") || lower.ends_with(".htm") {
                html::write_html(&mesh, path)
            } else {
                stl::write_binary(&mesh, path)
            };
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
            let (sdf, out, view) =
                if args.get(2).map(|s| s.ends_with(".json")).unwrap_or(false) {
                    let src = load_scene_file(args.get(2).unwrap());
                    let sdf = parse_scene(&src);
                    let out = args.get(3).map(String::as_str).unwrap_or("kado-screenshot.png");
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
            let min_wall: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.5);
            let max_overhang: f64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(45.0);
            let res: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(48);
            let res = res.clamp(1, 256);
            // 問47: run と同じヘルパを使い、ファイル読み込み・評価のエラー処理を一本化する。
            let sdf = parse_scene(&load_scene_file(path));
            let (lo, hi) = sdf.sampling_box();
            let mesh = polygonize(&sdf, lo, hi, res);
            // SDF を渡し局所薄肉の内向きレイ探針を有効化する (問58)。ビルド方向 +Z (問68)。
            let report = validate_with_field(&mesh, Some(&sdf), min_wall, max_overhang, Vec3::new(0.0, 0.0, 1.0));
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
            eprintln!("usage: kado [version|selftest|export|screenshot|run|check|mcp]");
            std::process::exit(2);
        }
    }
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
