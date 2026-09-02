//! ソース走査によるガード群 (問316/327/334)。
//!
//! いずれも「`src/` を実行時に再帰走査して、コードが満たすべき性質を確かめる」形を取る。
//! `include_str!` ではなく `read_dir` を使うのは**まだ存在しないファイル**にも効かせるため。
//! 検出語は実行時の文字列連結で組み立てる（リテラルで書くと走査対象に自分が入ったとき
//! 常に落ちる・問316/320 で二度学んだ）。
//!
//! ファイル名は最初のガード（C1）に由来する。改名しないのは、`SECURITY.md` §1 と
//! 問316 以降の議事録がこの名前で参照しているためで、**公開済みの記録の側を
//! 後から書き換えない**（問333 と同じ理由）。
//!
//! # C1「外部送信ゼロ」の契約化 (問316)
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

// ── 引数のサイレント強制変換の禁止 (問327) ──────────────────────────────────────

/// 引数を黙って既定値へ倒すパターンが、**製品コードに二度と現れない**ことを守る。
///
/// 問318（MCP の整数）・問325（CLI の整数）・問326（MCP の浮動小数/文字列/真偽値）は、
/// **三回に分けてしか行き渡らなかった同じ一つの規律**である。毎回「直した」と書いたが、
/// 直していたのは常に「見つけた場所」だけで、次の場所は次のラウンドまで残った。
///
/// マスク第1段階（要件を疑う）を自分の直し方に当てると、真の要件は
/// 「見つけた引数を直す」ではなく **「どの入口も引数を黙って読み替えられない」** である。
/// 個別修正は要件の実装のひとつにすぎず、**列挙し切ったことを誰も保証していなかった**。
/// これはガードで表せる——表せるものを人間の注意に委ねない（問320 の教訓）。
///
/// 検出するのは `x.get(..).and_then(|v| v.as_TYPE()).unwrap_or(default)` の連鎖、
/// すなわち「取り出せなければ黙って既定」の署名そのもの。`match` / `if let` で
/// 明示エラーにする正当な経路（`tool_eval` の必須 x/y/z など）は `unwrap_or` を
/// 使わないので引っかからない。
fn silent_coercion_signature(normalized: &str) -> Vec<String> {
    // 検出語は実行時の文字列連結で組み立てる（問316/問320 の教訓: リテラルで書くと
    // 走査対象に自分自身が入ったとき常に落ちる）。
    let unwrap_or = ["unwrap", "_or("].concat();
    let and_then = ["and", "_then(|"].concat();
    let as_prefix = [".as", "_"].concat();

    let mut found = Vec::new();
    let mut from = 0;
    while let Some(i) = normalized[from..].find(&and_then) {
        let start = from + i;
        // 連鎖は 1 式で完結するので、直後の限られた窓だけを見る。
        // 日本語コメントを含むため**文字単位**で切り出す（バイト添字だと
        // マルチバイト文字の途中で切れて panic する。実際に一度落とした）。
        let window: String = normalized[start..].chars().take(160).collect();
        // ".as_" と "unwrap_or(" が、この順にこの窓へ収まっていれば署名一致。
        if let (Some(a), Some(u)) = (window.find(&as_prefix), window.find(&unwrap_or)) {
            if a < u {
                found.push(window[..u + unwrap_or.len()].to_string());
            }
        }
        from = start + and_then.len();
    }
    found
}

/// ファイル本文のうち、テストモジュールより**前**（＝製品コード）だけを返す。
///
/// テストが JSON レポートを読むのに `unwrap_or("")` を使うのは正当なので、
/// 走査対象から外す。守りたいのは利用者の入力を解釈する経路だけである。
fn production_part(text: &str) -> &str {
    match text.find(&["#[cfg(", "test)]"].concat()) {
        Some(i) => &text[..i],
        None => text,
    }
}

#[test]
fn no_production_code_silently_coerces_an_argument_to_a_default() {
    let mut files = Vec::new();
    rust_sources(&src_dir(), &mut files);
    assert!(
        files.len() >= 20,
        "src/ の走査が壊れている ({} 件)",
        files.len()
    );

    let mut violations = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("読めない: {}: {e}", path.display()));
        // rustfmt が連鎖を複数行へ折るため、空白を潰してから探す。
        let normalized = production_part(&text)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        for hit in silent_coercion_signature(&normalized) {
            violations.push(format!("{}: {hit}", path.display()));
        }
    }

    assert!(
        violations.is_empty(),
        "引数を黙って既定値へ倒すパターンが製品コードに現れた (問318/325/326 の再発)。\n\
         省略は既定でよいが、**指定されたのに解釈できない値**を黙って読み替えてはならない。\n\
         `arg_f64` / `arg_str` / `arg_bool` / `arg_bounded_usize` (src/mcp/tools.rs) か、\n\
         `parse_finite_arg` / `parse_resolution_arg` (src/cli/main.rs) を使うこと。\n{}",
        violations.join("\n")
    );
}

/// 上のガードが**実際に違反を捕まえる**ことの確認（問314 以来の規律）。
#[test]
fn the_coercion_guard_detects_a_planted_silent_fallback() {
    let planted = [
        "let v = args",
        ".get(\"min_wall_mm\").and_then(|v| v",
        ".as_f64()).unwrap_or(0.5);",
    ]
    .concat();
    assert!(
        !silent_coercion_signature(&planted).is_empty(),
        "仕込んだサイレントフォールバックを検出できなかった"
    );

    // 明示エラーにする正当な経路は誤検出しない。
    for clean in [
        "let x = args.get(\"x\").and_then(|v| v.as_f64()); match x { Some(v) => v, None => return err }",
        "let n = self.count.unwrap_or(0);",
        "let s = text.find(\"x\").unwrap_or(0);",
    ] {
        assert!(
            silent_coercion_signature(clean).is_empty(),
            "正当な経路を違反と誤検出した: {clean}"
        );
    }
}

// ── SPEC のリソース上限表とソースの一致 (問334) ──────────────────────────────

/// `docs/SPEC.md` §7.4 の上限表が、実際の `const MAX_*` と一致していることを守る。
///
/// 問325 で `MAX_RESOLUTION` を `mcp/tools` から `extract` へ移したとき、**SPEC の
/// 「場所」列を更新し忘れた**。コードは正しく、文書だけが古い——本セッションで
/// 何度も見た形（問324 の本数、問329 の KPI、問328 の解像度）であり、
/// **私はその都度「手で同期する数値は腐る」と書きながら、また一つ腐らせていた**。
///
/// 検証するのは 2 つ:
/// 1. **網羅性** — `src/` に定義された `const MAX_*` はすべて表に載っていること。
///    新しい上限を足して文書化を忘れる経路を塞ぐ。
/// 2. **場所の正しさ** — 表の「場所」列が、実際に定義されているファイルを指すこと。
///
/// **値の列は検証していない。** `1 MiB` と `1 << 20`、`16 MiB` と `16 * 1024 * 1024` を
/// 突き合わせるには両側に式評価器が要る。値は（本数や KPI と違い）めったに、そして
/// 意図的にしか変わらないので、その複雑さに見合わない。測っていないことを
/// 測ったと書かないためにここへ明記する（問331）。
#[test]
fn spec_resource_limit_table_matches_the_source() {
    let spec = include_str!("../docs/SPEC.md");

    // src/ から `const MAX_*` を集める (名前 → モジュールパス)。
    let mut files = Vec::new();
    rust_sources(&src_dir(), &mut files);
    assert!(
        files.len() >= 20,
        "src/ の走査が壊れている ({} 件)",
        files.len()
    );

    let needle = ["const ", "MAX_"].concat();
    let mut defined: Vec<(String, String)> = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path).expect("read source");
        // src/a/b.rs → "a/b"、src/a/mod.rs → "a"
        let rel = path.strip_prefix(src_dir()).expect("under src/");
        let mut module = rel.with_extension("").to_string_lossy().replace('\\', "/");
        if let Some(base) = module.strip_suffix("/mod") {
            module = base.to_string();
        }
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("//") {
                continue;
            }
            if let Some(i) = line.find(&needle) {
                let rest = &line[i + "const ".len()..];
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
                    .collect();
                if name.starts_with("MAX_")
                    && !defined.iter().any(|(n, m)| *n == name && *m == module)
                {
                    defined.push((name, module.clone()));
                }
            }
        }
    }
    assert!(
        defined.len() >= 8,
        "上限定数の抽出が壊れている (見つかったのは {:?})",
        defined
    );

    for (name, module) in &defined {
        // 表の行は `| \`NAME\` | 値 | 場所 |`。同名が別モジュールにある場合
        // (MAX_DEPTH は script/eval と mcp/json) は、場所を含む行があればよい。
        let rows: Vec<&str> = spec
            .lines()
            .filter(|l| l.trim_start().starts_with('|') && l.contains(&format!("`{name}`")))
            .collect();
        assert!(
            !rows.is_empty(),
            "`{name}` ({module}) が docs/SPEC.md §7.4 の上限表に無い。\n\
             新しいリソース上限を足したら表にも書くこと (問334)"
        );
        assert!(
            rows.iter().any(|r| r.contains(module.as_str())),
            "docs/SPEC.md の `{name}` の行が定義場所 '{module}' を指していない (問334)。\n\
             定数を移動したら「場所」列も直すこと——問325 で実際に忘れた。\n{rows:#?}"
        );
    }
}
