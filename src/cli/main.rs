//! Kado CLI (Phase 2 で `run/render/check/export` を本実装予定)。
//!
//! 現時点の足場: version / selftest / export(デモ形状をSTL出力)。

use kado::core::{Sdf, Vec3};
use kado::extract::polygonize;
use kado::io::stl;

/// デモ形状: 球 ∪ 直方体 − 円柱 (穴あき)。スパイク用の代表モデル。
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
            let mesh = polygonize(&demo_model(), Vec3::splat(-1.5), Vec3::splat(1.5), 64);
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
        other => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: kado [version|selftest|export <out.stl>]");
            std::process::exit(2);
        }
    }
}
