//! 利用者に見える**約束**の検証 (問321/332/333)。
//!
//! CLI の振る舞いと、ドキュメントが新規利用者に約束していることを、
//! どちらも**実行・検査して**確かめる。両者は同じ性質のものである——
//! 「そう書いてある」ことと「そうなっている」ことは別であり、後者だけが約束である。
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

/// README のクイックスタートを**実際に実行する** (問332)。
///
/// 問321 は「クイックスタートを一文字変えた」だけでデータ破壊バグを暴いた。ならば
/// **一文字も変えずに実行**したらどうか——最後の `check` が**終了コード 1** で終わっていた。
/// README は `r: 1.0`（直径 2mm の球）に `min_wall 0.8mm` を課しており、平均肉厚
/// 0.666mm < 0.8mm で正しく FAIL する。判定は正しいが、**手順どおりに進んだ利用者が
/// 赤い `[Error]` で終わる**のは案内として誤りである（利用者は何も間違えていない）。
///
/// README は新規利用者への約束である。約束は実行して確かめる。
#[test]
fn the_readme_quickstart_actually_runs_and_succeeds() {
    let readme = include_str!("../README.md");
    // 最初の ```sh ブロック＝クイックスタート。
    let block = readme
        .split("```sh")
        .nth(1)
        .and_then(|b| b.split("```").next())
        .expect("README must have a shell quickstart block");

    let dir = workdir("readme-quickstart");
    // ブロックは `./target/release/kado` を叩く。そのパスに実バイナリを置いて
    // **コマンドを一字も書き換えずに**実行する（書き換えれば README を検証したことに
    // ならない）。
    let bin_dir = dir.join("target").join("release");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::copy(env!("CARGO_BIN_EXE_kado"), bin_dir.join("kado")).unwrap();

    let mut ran = 0;
    for line in block.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("cargo ") {
            continue; // コメントと、既にビルド済みの cargo build は飛ばす
        }
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(line)
            .current_dir(&dir)
            .output()
            .unwrap_or_else(|e| panic!("failed to run quickstart line `{line}`: {e}"));
        assert!(
            out.status.success(),
            "README quickstart line failed with {:?} (問332):\n  $ {line}\n  stdout: {}\n  stderr: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
        ran += 1;
    }
    assert!(
        ran >= 4,
        "the quickstart should exercise several commands; only {ran} ran — the block parse \
         is probably broken and this test would pass vacuously"
    );
}

/// `CHANGELOG.md` が Keep a Changelog の構造を保っていることを守る (問333)。
///
/// CHANGELOG 冒頭は「形式は Keep a Changelog に従う」と宣言している。だがこの宣言には
/// **強制手段が無かった**——本セッション中、私は自分の編集で
/// `## [Unreleased]` 見出しを**2 度**消し（問321 と問331）、`###` 見出しを**2 度**重複させた。
/// 1 度目は問324 で偶然気づいたが、2 度目は問332 の作業中に見つかるまで残っていた。
///
/// 見出しの欠落は静かに壊れる。バレットは見出しが無くても表示され、
/// リリースノート生成器だけが後で困る。**気づく仕組みが無ければ、
/// 同じ人間が同じ間違いを繰り返す**（問320/327 と同じ結論を、自分に適用する）。
#[test]
fn changelog_keeps_the_structure_it_claims_to_follow() {
    let text = include_str!("../CHANGELOG.md");
    assert!(
        text.contains("Keep a Changelog"),
        "CHANGELOG must declare the format it follows"
    );
    assert!(
        text.contains("## [Unreleased]"),
        "CHANGELOG must have an [Unreleased] section — it was silently deleted twice by \
         careless edits during this session (問321/問331)"
    );

    // Keep a Changelog が定める変更種別。これ以外の `###` は誤記とみなす。
    const KINDS: &[&str] = &["追加", "変更", "修正", "削除", "非推奨", "セキュリティ"];

    let mut release: Option<&str> = None;
    let mut kind: Option<&str> = None;
    let mut seen_in_release: Vec<&str> = Vec::new();

    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if let Some(title) = line.strip_prefix("## ") {
            release = Some(title.trim());
            kind = None;
            seen_in_release.clear();
        } else if let Some(k) = line.strip_prefix("### ") {
            let k = k.trim();
            let rel = release.unwrap_or_else(|| panic!("line {n}: `### {k}` before any release"));
            // 種別の語彙は [Unreleased] にのみ課す。v0.1.0 は変更種別ではなく
            // サブシステム（幾何カーネル・入出力…）で束ねており、**公開済みの履歴を
            // 後から書き換えるのは誤り**である。ガードは今後書く場所にだけ効かせる
            // ——過度に厳しいガードはノイズを生み、ノイズを出すガードは無視される（問320）。
            if rel.contains("Unreleased") {
                assert!(
                    KINDS.contains(&k),
                    "line {n}: `### {k}` in {rel} is not a Keep a Changelog kind {KINDS:?}"
                );
            }
            assert!(
                !seen_in_release.contains(&k),
                "line {n}: `### {k}` appears twice in {rel} — merge the sections (問333)"
            );
            seen_in_release.push(k);
            kind = Some(k);
        } else if line.starts_with("- ") {
            // バレットは必ず「リリース → 種別」の下に置かれること。
            assert!(
                release.is_some() && kind.is_some(),
                "line {n}: entry sits outside a `## release` / `### kind` heading — a heading \
                 was probably deleted by an edit (問333): {line}"
            );
        }
    }
}
