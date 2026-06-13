//! Kado CLI。
//!
//! コマンド:
//!   version    バージョン表示
//!   selftest   最小 SDF 評価の動作確認
//!   export     [scene.json] <out.stl>  STL 出力 (JSON 省略時はデモモデル)
//!   screenshot [scene.json] <out.png> [view]  PNG スクリーンショット出力
//!   run        <scene.json>  メッシュ統計表示
//!   check      <scene.json> [min_wall_mm] [max_overhang_deg]  DFM 検証
//!   mcp        MCP サーバー (stdio) 起動

use kado::core::{Sdf, Vec3};
use kado::extract::polygonize;
use kado::io::stl;
use kado::mcp::server::run_stdio;
use kado::render::{render, Camera};
use kado::script::eval_scene;
use kado::verify::validate;

fn demo_model() -> Sdf {
    Sdf::sphere(1.0)
        .union(Sdf::cuboid(Vec3::splat(0.8)))
        .difference(Sdf::cylinder(0.3, 2.0))
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
            match stl::write_binary(&mesh, std::path::Path::new(&out)) {
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
            let cam = presets
                .iter()
                .find(|(n, _)| *n == view.as_str())
                .or_else(|| presets.iter().find(|(n, _)| *n == "iso"))
                .unwrap_or(&presets[0])
                .1
                .clone();
            let img = render(&mesh, &cam, 512, 512);
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
            let path = args.get(2).map(String::as_str).unwrap_or_else(|| {
                eprintln!("usage: kado run <scene.json>");
                std::process::exit(2);
            });
            let src = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("cannot read {path}: {e}");
                    std::process::exit(1);
                }
            };
            let sdf = match eval_scene(&src) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("script error: {e}");
                    std::process::exit(1);
                }
            };
            let (lo, hi) = sdf.sampling_box();
            let mesh = polygonize(&sdf, lo, hi, 48);
            let report = validate(&mesh, 0.0, 0.0);
            println!("{}", report.summary());
        }
        "check" => {
            let path = args.get(2).map(String::as_str).unwrap_or_else(|| {
                eprintln!("usage: kado check <scene.json> [min_wall_mm] [max_overhang_deg]");
                std::process::exit(2);
            });
            let min_wall: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.5);
            let max_overhang: f64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(45.0);
            let src = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("cannot read {path}: {e}");
                    std::process::exit(1);
                }
            };
            let sdf = match eval_scene(&src) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("script error: {e}");
                    std::process::exit(1);
                }
            };
            let (lo, hi) = sdf.sampling_box();
            let mesh = polygonize(&sdf, lo, hi, 48);
            let report = validate(&mesh, min_wall, max_overhang);
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
    match eval_scene(src) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("script error: {e}");
            std::process::exit(1);
        }
    }
}
