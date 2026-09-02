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

#[test]
fn export_rejects_an_unknown_extension_instead_of_writing_a_mislabelled_file() {
    // 問322: 以前は未知の拡張子を黙って STL にフォールバックしていたため、
    // `export model.obj` は「中身は binary STL・名前は .obj」というファイルを
    // 無言で作った。利用者は Kado が OBJ を書けたと信じ、別のツールで開けない
    // 理由が分からない。実バイナリでも拒否されることを固定する。
    let dir = workdir("badext");
    std::fs::write(dir.join("scene.txt"), SCENE).unwrap();

    let out = kado(&dir, &["export", "scene.txt", "model.obj"]);
    assert!(
        !out.status.success(),
        "an unsupported output format must fail loudly"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains(".obj") && err.contains(".stl") && err.contains(".glb"),
        "the error must name the bad extension and list the supported ones: {err}"
    );
    assert!(
        !dir.join("model.obj").exists(),
        "nothing must be written when the format is rejected"
    );

    // 拡張子が無い場合は STL 既定のまま (拒否するのは入力が誤っている場合だけ)。
    let ok = kado(&dir, &["export", "scene.txt", "noext"]);
    assert!(
        ok.status.success(),
        "a path without an extension must still default to STL: {}",
        String::from_utf8_lossy(&ok.stderr)
    );
    assert!(dir.join("noext").exists());
}

#[test]
fn run_and_check_reject_a_malformed_resolution_instead_of_silently_clamping() {
    // 問325: 問318 で MCP から消したサイレントフォールバックが CLI に残っていた。
    // `run scene.txt 100000` は黙って 256 に、`run scene.txt abc` は黙って 48 になり、
    // 利用者は頼んだ解像度の結果だと信じる。同じ `check` の中で `min_wall_mm` は
    // 既に厳格だったのに、3 引数のうち resolution だけ漏れていた。
    let dir = workdir("resolution");
    std::fs::write(dir.join("scene.txt"), SCENE).unwrap();

    for (args, needle) in [
        (&["run", "scene.txt", "100000"][..], "256"),
        (&["run", "scene.txt", "abc"][..], "integer"),
        (&["run", "scene.txt", "0"][..], "256"),
        (&["check", "scene.txt", "0.8", "45", "99999"][..], "256"),
    ] {
        let out = kado(&dir, args);
        assert!(
            !out.status.success(),
            "{args:?}: a malformed resolution must fail, not silently clamp (問325)"
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("resolution") && err.contains(needle),
            "{args:?}: error must name the argument and the accepted range: {err}"
        );
    }

    // 正常値と省略は従来どおり通る (厳格化で正常経路を壊していないこと)。
    for args in [
        &["run", "scene.txt", "24"][..],
        &["run", "scene.txt"][..],
        &["check", "scene.txt", "0.8", "45", "32"][..],
    ] {
        let out = kado(&dir, args);
        // check は DFM 結果次第で exit 1 になりうるので、引数エラーの exit 2 でないことを見る。
        assert_ne!(
            out.status.code(),
            Some(2),
            "{args:?}: valid resolution must not be an argument error: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn run_and_check_report_the_resolution_behind_their_numbers() {
    // 問328: `digest` も `min_wall` も**解像度に依存する**。MCP 側は問90/91/92 で
    // resolution を併記するようにしたが、CLI にはその修正が届いていなかった
    // (問325 と同じ「片方の入口にだけ適用された規律」)。同じファイルを 2 つの解像度で
    // 叩くと digest も min_wall も変わるのに、出力にはその理由が無かった。
    let dir = workdir("resolution-transparency");
    std::fs::write(dir.join("scene.txt"), SCENE).unwrap();

    let mut digests = Vec::new();
    for res in ["48", "96"] {
        for cmd in [
            vec!["run", "scene.txt", res],
            vec!["check", "scene.txt", "0.0", "0.0", res],
        ] {
            let out = kado(&dir, &cmd);
            let text = String::from_utf8_lossy(&out.stdout);
            assert!(
                text.contains(&format!("resolution={res}")),
                "{cmd:?} must state the resolution behind its digest/min_wall (問328): {text}"
            );
            if cmd[0] == "run" {
                let d = text
                    .split_whitespace()
                    .find(|t| t.starts_with("digest="))
                    .expect("summary must carry a digest")
                    .to_string();
                digests.push(d);
            }
        }
    }
    // 前提そのものの確認: 解像度が変われば digest は実際に変わる。
    // 変わらないなら「解像度を併記する」動機自体が成り立たない。
    assert_ne!(
        digests[0], digests[1],
        "digest must actually differ across resolutions — otherwise there is nothing to explain"
    );
}

#[test]
fn validate_stl_does_not_claim_a_resolution_it_does_not_have() {
    // 問328: `validate-stl` はファイルから読んだメッシュを検証する。抽出解像度という
    // 概念が無いので、ここに resolution を出すのは**嘘になる**。厳格化や透明性の
    // 勢いで、意味のない数値を足していないことを固定する。
    let dir = workdir("stl-no-resolution");
    std::fs::write(dir.join("scene.txt"), SCENE).unwrap();
    assert!(kado(&dir, &["export", "scene.txt", "m.stl"])
        .status
        .success());

    let out = kado(&dir, &["validate-stl", "m.stl", "0.0", "0.0"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("triangles="),
        "validate-stl must still print a summary: {text}"
    );
    assert!(
        !text.contains("resolution="),
        "an imported mesh has no extraction resolution; reporting one would be a lie (問328): {text}"
    );
}
