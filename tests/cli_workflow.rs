//! CLI エンドツーエンド統合テスト (問321)。
//!
//! 問321 で `kado export scene.txt out.stl` が**利用者のシーンファイルへ STL を
//! 上書きする**欠陥が見つかった。ユニットテストは各層を検証していたが、
//! CLI の引数解決を実バイナリで通す経路が無かった (MCP 側には `mcp_workflow.rs` が
//! あるのに、CLI 側には対応するものが無かった——`docs/SPEC.md` §10 の
//! 「ユーザーが実際に叩く経路で E2E 検証する」が片肺だった)。
//!
//! 破壊的な欠陥は「壊れないこと」を実際に観測して初めて塞いだと言える。

use std::path::Path;
use std::process::Command;

/// テスト専用の作業ディレクトリを作る（テストは並列実行されるので名前を分ける）。
fn workdir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kado-cli-test-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create workdir");
    dir
}

fn kado(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kado"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run kado")
}

const SCENE: &str = "difference(flatten(0, sphere(10.0)), cylinder(1.6, 25.0))";

#[test]
fn export_writes_the_output_and_leaves_the_scene_file_untouched() {
    let dir = workdir("export");
    let scene = dir.join("scene.txt");
    std::fs::write(&scene, SCENE).unwrap();

    let out = kado(&dir, &["export", "scene.txt", "out.stl"]);
    assert!(
        out.status.success(),
        "export failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 1. 出力が指定した名前で作られること (以前は無視されていた)。
    let stl = dir.join("out.stl");
    assert!(
        stl.exists(),
        "export must write the path it was given; instead it wrote nothing there (問321)"
    );
    assert!(
        std::fs::metadata(&stl).unwrap().len() > 84,
        "STL must contain triangles, not just a header"
    );

    // 2. シーンファイルが**一字一句そのまま**であること。
    //    以前はここへ 3.5MB の STL が上書きされ、原本が復元不能に失われた。
    assert_eq!(
        std::fs::read_to_string(&scene).unwrap(),
        SCENE,
        "the scene file must never be overwritten by the export (問321)"
    );
}

#[test]
fn screenshot_writes_the_output_and_leaves_the_scene_file_untouched() {
    let dir = workdir("screenshot");
    let scene = dir.join("scene.txt");
    std::fs::write(&scene, SCENE).unwrap();

    let out = kado(&dir, &["screenshot", "scene.txt", "shot.png", "top"]);
    assert!(
        out.status.success(),
        "screenshot failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let png = dir.join("shot.png");
    assert!(png.exists(), "screenshot must write the path it was given");
    // PNG シグネチャ (デモモデルではなく指定シーンが描かれたことまでは、
    // 上書きされていないシーンから別途検証される)。
    let bytes = std::fs::read(&png).unwrap();
    assert_eq!(
        &bytes[..8],
        b"\x89PNG\r\n\x1a\n",
        "output must be a real PNG"
    );
    assert_eq!(
        std::fs::read_to_string(&scene).unwrap(),
        SCENE,
        "the scene file must never be overwritten by the screenshot (問321)"
    );
}

#[test]
fn export_of_a_scene_differs_from_the_demo_model() {
    // 問321 の二次被害: シーンを出力パスと誤認すると、**デモモデル**が
    // 書き出されていた。利用者は自分のシーンを出力したつもりで、
    // 全く別の形状を受け取る。両者が実際に違うことを固定しておく。
    let dir = workdir("distinct");
    std::fs::write(dir.join("scene.txt"), SCENE).unwrap();

    assert!(kado(&dir, &["export", "scene.txt", "mine.stl"])
        .status
        .success());
    assert!(kado(&dir, &["export", "demo.stl"]).status.success());

    let mine = std::fs::read(dir.join("mine.stl")).unwrap();
    let demo = std::fs::read(dir.join("demo.stl")).unwrap();
    assert_ne!(
        mine, demo,
        "exporting a scene must not silently produce the demo model (問321)"
    );
}

#[test]
fn export_refuses_to_overwrite_the_scene_file() {
    // 解析は直したが、上書きは取り返しがつかないので防御を一枚残してある。
    let dir = workdir("clobber");
    let scene = dir.join("scene.txt");
    std::fs::write(&scene, SCENE).unwrap();

    let out = kado(&dir, &["export", "scene.txt", "scene.txt"]);
    assert!(
        !out.status.success(),
        "writing the output over the scene file must fail loudly"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("refusing"),
        "the error must say why: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&scene).unwrap(),
        SCENE,
        "the scene file must survive the refused command"
    );
}
