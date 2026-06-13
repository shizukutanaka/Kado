//! Kado CLI。
//!
//! コマンド:
//!   version    バージョン表示
//!   selftest   最小 SDF 評価の動作確認
//!   export     STL ファイル出力
//!   screenshot PNG スクリーンショット出力
//!   mcp        MCP サーバー (stdio) 起動

use kado::core::{Sdf, Vec3};
use kado::extract::polygonize;
use kado::io::stl;
use kado::mcp::server::run_stdio;
use kado::render::{render, Camera};

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
            let out = args.get(2).map(String::as_str).unwrap_or("kado-demo.stl");
            let mesh = polygonize(&demo_model(), Vec3::splat(-2.0), Vec3::splat(2.0), 64);
            match stl::write_binary(&mesh, std::path::Path::new(out)) {
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
            let out = args
                .get(2)
                .map(String::as_str)
                .unwrap_or("kado-screenshot.png");
            let view = args.get(3).map(String::as_str).unwrap_or("iso");
            let mesh = polygonize(&demo_model(), Vec3::splat(-2.0), Vec3::splat(2.0), 48);
            if mesh.triangles.is_empty() {
                eprintln!("mesh is empty");
                std::process::exit(1);
            }
            let (lo, hi) = mesh.bounds().unwrap();
            let presets = Camera::presets(lo, hi);
            let cam = presets
                .iter()
                .find(|(n, _)| *n == view)
                .unwrap_or(&presets[6])
                .1
                .clone();
            let img = render(&mesh, &cam, 512, 512);
            match img.write_png(std::path::Path::new(out)) {
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
        "mcp" => {
            // MCP サーバーモード (stdin/stdout・返らない)。
            run_stdio();
        }
        other => {
            eprintln!("unknown command: {other}");
            eprintln!(
                "usage: kado [version|selftest|export <out.stl>|screenshot <out.png> [view]|mcp]"
            );
            std::process::exit(2);
        }
    }
}
