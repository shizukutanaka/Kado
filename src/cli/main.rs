//! Kado CLI (Phase 2 で `run/render/check/export` を実装予定)。
//!
//! 現時点では足場のみ: バージョン表示と、SDF木が評価可能であることの最小確認。

use kado::core::{Sdf, Vec3};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("version");

    match cmd {
        "version" | "--version" | "-V" => {
            println!("kado {}", env!("CARGO_PKG_VERSION"));
        }
        "selftest" => {
            // 最小E2E: SDF木を1点で評価して非破綻を確認する。
            let tree = Sdf::sphere(1.0)
                .union(Sdf::cuboid(Vec3::splat(0.7)))
                .difference(Sdf::cylinder(0.3, 2.0));
            let d = tree.eval(Vec3::ZERO);
            println!("selftest ok: f(origin) = {d}");
        }
        other => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: kado [version|selftest]");
            std::process::exit(2);
        }
    }
}
