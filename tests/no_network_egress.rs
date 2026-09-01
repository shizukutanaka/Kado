//! C1「外部送信ゼロ」の契約化 (問316)。
//!
//! `SECURITY.md` §1 は「ネットワークソケットを一切開かない」と宣言しているが、
//! この主張だけ**強制手段が無かった**。`no-external-deps` ジョブ (`cargo tree`) が
//! 守っているのは C2「外部 crate ゼロ」であり、ネットワーク型は**標準ライブラリの
//! 一部**なので依存ツリーには現れない。つまり `C2 ⇒ C1` は成り立たず、
//! ソケット型を1行足しても既存のゲートは1つも落ちない状態だった。
//!
//! 本テストは `src/` を実行時に再帰走査してネットワーク API の参照を禁じる。
//! `include_str!` ではなく `read_dir` を使うのは、**まだ存在しないファイル**にも
//! ガードを効かせるため（`include_str!` は1ファイルずつ列挙する必要があり、
//! 列挙漏れがそのまま穴になる）。

use std::path::{Path, PathBuf};

/// 禁止トークンを**実行時の文字列連結**で組み立てる。
///
/// ソース上にリテラルとして現れないため、このテスト自身が走査対象に入っても
/// 自分に反応しない。結果として除外リストが不要になり、将来このファイルを
/// `src/` 配下へ移しても壊れない (除外リストは腐る典型)。
fn forbidden_tokens() -> Vec<String> {
    let net = ["::", "net"].concat();
    vec![
        ["std", &net].concat(),
        ["Tcp", "Stream"].concat(),
        ["Tcp", "Listener"].concat(),
        ["Udp", "Socket"].concat(),
        ["Socket", "Addr"].concat(),
        ["To", "Socket", "Addrs"].concat(),
        ["Ip", "Addr"].concat(),
    ]
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("走査できない: {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .collect();
    // read_dir の順序は OS 依存。決定的な報告のためにソートする (問5)。
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

#[test]
fn no_source_file_references_network_apis() {
    let mut files = Vec::new();
    rust_sources(&src_dir(), &mut files);

    // 走査が壊れて0件になれば「違反ゼロ」で無言合格してしまう。
    // 実際に読めていることを先に固定する (問314 の教訓: 通るだけのガードは証拠にならない)。
    assert!(
        files.len() >= 20,
        "src/ の走査結果が少なすぎる ({} 件)。走査が壊れている可能性がある",
        files.len()
    );
    assert!(
        files.iter().any(|p| p.ends_with("lib.rs")),
        "src/lib.rs が走査結果に含まれていない"
    );

    let tokens = forbidden_tokens();
    let mut violations = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("読めない: {}: {e}", path.display()));
        for (lineno, line) in text.lines().enumerate() {
            for token in &tokens {
                if line.contains(token.as_str()) {
                    violations.push(format!(
                        "{}:{}: {} — {}",
                        path.display(),
                        lineno + 1,
                        token,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "C1「外部送信ゼロ」(SECURITY.md §1) 違反: ネットワーク API を参照している。\n\
         Kado は stdio のみで通信する。ソケットを開く必要が生じた場合は、\n\
         まず SPEC の非目標を見直すこと (この行を消して通すのは契約の破棄にあたる)。\n{}",
        violations.join("\n")
    );
}

/// 走査ロジック自体が「違反を見つけられる」ことの実地確認。
///
/// 本物の違反を仕込む訳にはいかないので、同じ検出関数を一時ファイルに向けて走らせ、
/// 検出されることを確かめる。これが無いと、検出条件を壊しても
/// [`no_source_file_references_network_apis`] は静かに通り続ける。
#[test]
fn the_guard_actually_detects_a_planted_violation() {
    let tokens = forbidden_tokens();
    // 違反行そのものも連結で組み立てる。ここにリテラルを置くと、このファイルを
    // `src/` 配下へ移した瞬間に自分自身を違反として検出してしまう。
    let planted = format!(
        "let s = {}::{}::connect(addr);",
        ["std", "::", "net"].concat(),
        ["Tcp", "Stream"].concat()
    );
    assert!(
        tokens.iter().any(|t| planted.contains(t.as_str())),
        "仕込んだ違反行を検出できなかった。検出トークンが壊れている"
    );

    let clean = "let v = self.eval(p) + 1.0; // 純粋な幾何計算";
    assert!(
        !tokens.iter().any(|t| clean.contains(t.as_str())),
        "無害な行を違反と誤検出した"
    );
}

/// このガード自身が禁止トークンを1つも含まないこと。
///
/// [`forbidden_tokens`] の doc は「`src/` 配下へ移しても壊れない」と主張する。
/// **主張をコメントに書くだけでは守られない**（本ラウンド問316 の主題そのもの）ので、
/// テストで固定する。実際、この検査を入れて初めて、検出語の1つが**別の検出語に
/// 部分一致して自分に反応する**ことが判明した（複数形の綴りが単数形を含んでいた）。
/// 検出語を分割する規律は、目視ではなく検算でしか保てない。
#[test]
fn the_guard_itself_contains_no_forbidden_token() {
    let me = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("no_network_egress.rs");
    let text =
        std::fs::read_to_string(&me).unwrap_or_else(|e| panic!("読めない: {}: {e}", me.display()));

    let hits: Vec<String> = text
        .lines()
        .enumerate()
        .flat_map(|(i, line)| {
            forbidden_tokens()
                .into_iter()
                .filter(move |t| line.contains(t.as_str()))
                .map(move |t| format!("{}: {t}", i + 1))
        })
        .collect();

    assert!(
        hits.is_empty(),
        "ガード自身が禁止トークンを含む。`src/` へ移すと自分に反応して常に落ちる。\n\
         検出語は実行時の文字列連結で組み立てること。複数形の綴りが単数形の検出語に\n\
         部分一致する例があるため、分割位置は検算で決めること。\n{}",
        hits.join("\n")
    );
}
