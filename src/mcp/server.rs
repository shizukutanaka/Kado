//! MCP stdio サーバー。
//!
//! トランスポート: Content-Length フレーミング (LSP スタイル)。
//! プロトコル: MCP 2024-11-05 / JSON-RPC 2.0。
//! `run_stdio()` は stdin/stdout をブロッキングで読み書きし、
//! 永続的に動作する (SIGPIPE または stdin EOF で終了)。

use std::io::{self, BufRead, Write};

use crate::mcp::json::{self, Value};
use crate::mcp::tools;

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "kado";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 1 フレームの本文バイト数上限 (問118: トランスポート層の DoS 防御)。
///
/// `Content-Length` は信頼できないクライアントから来る。これを無検査で
/// `vec![0u8; len]` の確保に使うと、`Content-Length: 999999999999` 一発で
/// OOM クラッシュ (= DoS) を起こせる。DSL 評価器の問16 リソース上限
/// (ソース 1 MiB) は**本文を全部メモリに読んだ後**にしか効かないため、確保前に
/// ここで遮断する。本文は JSON-RPC エンベロープ + エスケープでスクリプト生バイトより
/// 膨らむため、ソース上限 (1 MiB) に十分な余裕を見て 16 MiB を上限とする。
const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

/// stdio で MCP サーバーを起動する。返らない。
pub fn run_stdio() -> ! {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    // セッション状態 (正本シーン)。run_script で更新され他ツールが参照する。
    let mut session = tools::Session::new();

    // read_message が Err (stdin EOF または不正フレーム) を返すまで処理を続ける。
    while let Ok(msg) = read_message(&mut reader) {
        if let Some(resp) = handle(&mut session, &msg) {
            if write_message(&mut writer, &resp).is_err() {
                break;
            }
        }
    }
    std::process::exit(0)
}

// ── フレーミング ──────────────────────────────────────────────────────────────

fn read_message(r: &mut impl BufRead) -> io::Result<Value> {
    // ヘッダを行単位で読む。Content-Length: N\r\n\r\n の形式。
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = r.read_line(&mut line)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "stdin closed"));
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        } // 空行 = ヘッダ終端
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse().ok();
        }
    }
    let len = content_length
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    // 問118: 確保前にフレーム上限を検査し、巨大 Content-Length による OOM を防ぐ。
    if len > MAX_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Content-Length {len} exceeds limit {MAX_MESSAGE_BYTES}"),
        ));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    let text =
        std::str::from_utf8(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    json::parse(text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn write_message(w: &mut impl Write, v: &Value) -> io::Result<()> {
    let body = v.to_string();
    write!(w, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    w.flush()
}

// ── リクエストハンドラ ────────────────────────────────────────────────────────

fn handle(session: &mut tools::Session, msg: &Value) -> Option<Value> {
    let method = msg.get("method")?.as_str()?;
    let id = msg.get("id").cloned().unwrap_or(json::NULL);
    let params = msg.get("params").cloned().unwrap_or(json::NULL);

    // notifications (id なし) はレスポンス不要。
    let is_notification = msg.get("id").is_none();

    let result = match method {
        "initialize" => Some(handle_initialize(&params)),
        "initialized" => return None, // notification
        "tools/list" => Some(handle_tools_list()),
        "tools/call" => Some(handle_tools_call(session, &params)),
        "ping" => Some(json::obj([])),
        _ => {
            if is_notification {
                return None;
            }
            return Some(error_response(id, -32601, "Method not found"));
        }
    };

    result.map(|r| success_response(id, r))
}

fn handle_initialize(_params: &Value) -> Value {
    json::obj([
        ("protocolVersion", json::s(MCP_PROTOCOL_VERSION)),
        (
            "serverInfo",
            json::obj([
                ("name", json::s(SERVER_NAME)),
                ("version", json::s(SERVER_VERSION)),
            ]),
        ),
        (
            "capabilities",
            json::obj([("tools", json::obj([("listChanged", json::b(false))]))]),
        ),
    ])
}

fn handle_tools_list() -> Value {
    json::obj([("tools", tools::tool_list())])
}

fn handle_tools_call(session: &mut tools::Session, params: &Value) -> Value {
    // 問106: 不正な tools/call (name 欠落・非文字列) も、未知ツール (call_tool 経由) と
    // 同じ {content, isError:true} 形で返し、tools/call 応答の構造を統一する。
    // 以前は missing name だけ {code, message} を返し、それが success_response で
    // result に包まれて content/isError を欠く非標準応答になっていた。
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return tool_error_result("missing or non-string tool name"),
    };
    let empty = json::obj([]);
    let args = params.get("arguments").unwrap_or(&empty);

    let result = tools::call_tool(session, &name, args);
    json::obj([
        ("content", Value::Array(result.content)),
        ("isError", json::b(result.is_error)),
    ])
}

/// tools/call のエラーを標準形 {content:[text], isError:true} で返す (問106)。
fn tool_error_result(message: &str) -> Value {
    json::obj([
        (
            "content",
            json::arr([json::obj([
                ("type", json::s("text")),
                ("text", json::s(message)),
            ])]),
        ),
        ("isError", json::b(true)),
    ])
}

// ── JSON-RPC ヘルパ ───────────────────────────────────────────────────────────

fn success_response(id: Value, result: Value) -> Value {
    json::obj([("jsonrpc", json::s("2.0")), ("id", id), ("result", result)])
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json::obj([
        ("jsonrpc", json::s("2.0")),
        ("id", id),
        (
            "error",
            json::obj([
                ("code", json::n(code as f64)),
                ("message", json::s(message)),
            ]),
        ),
    ])
}


#[cfg(test)]
mod tests {
    use super::*;

    fn req(method: &str, id: i64, params: Option<Value>) -> Value {
        let mut m = std::collections::BTreeMap::new();
        m.insert("jsonrpc".into(), json::s("2.0"));
        m.insert("method".into(), json::s(method));
        m.insert("id".into(), json::n(id as f64));
        if let Some(p) = params {
            m.insert("params".into(), p);
        }
        Value::Object(m)
    }

    fn eval_at(session: &mut tools::Session, x: f64, y: f64, z: f64) -> f64 {
        let params = json::obj([
            ("name", json::s("eval")),
            (
                "arguments",
                json::obj([("x", json::n(x)), ("y", json::n(y)), ("z", json::n(z))]),
            ),
        ]);
        let resp = handle(session, &req("tools/call", 3, Some(params))).unwrap();
        let text = resp
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .unwrap()[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        text
    }

    #[test]
    fn initialize_returns_protocol_version() {
        let mut s = tools::Session::new();
        let resp = handle(&mut s, &req("initialize", 1, None)).unwrap();
        let ver = resp
            .get("result")
            .and_then(|r| r.get("protocolVersion"))
            .and_then(|v| v.as_str());
        assert_eq!(ver, Some(MCP_PROTOCOL_VERSION));
    }

    #[test]
    fn tools_list_has_eight_tools() {
        // 問67で undo_script ツールを追加した (screenshot, export, eval, run_script,
        // validate, get_scene, undo_script, help)。
        let mut s = tools::Session::new();
        let resp = handle(&mut s, &req("tools/list", 2, None)).unwrap();
        let tools = resp
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|v| v.as_array());
        assert_eq!(tools.map(|a| a.len()), Some(8));
    }

    #[test]
    fn get_scene_round_trip() {
        // 問26: run_script 後に get_scene でスクリプトを読み返せることを検証する。
        // コンテキスト消失後もAIがシーン状態を確認できる自己修正ループの要件。
        let mut s = tools::Session::new();

        // 初期状態: スクリプト未設定のデフォルトシーン。
        let params_get = json::obj([("name", json::s("get_scene")), ("arguments", json::obj([]))]);
        let resp = handle(&mut s, &req("tools/call", 10, Some(params_get.clone()))).unwrap();
        let text = resp
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .unwrap()[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(
            text.contains("default scene"),
            "before run_script, get_scene should note default: {text}"
        );

        // run_script で球を設定。
        let script = r#"{"op":"sphere","r":2.0}"#;
        let params_run = json::obj([
            ("name", json::s("run_script")),
            ("arguments", json::obj([("script", json::s(script))])),
        ]);
        let run_resp = handle(&mut s, &req("tools/call", 11, Some(params_run))).unwrap();
        assert_eq!(
            run_resp.get("result").and_then(|r| r.get("isError")),
            Some(&json::b(false))
        );

        // get_scene が同じスクリプトを返すことを確認。
        let resp2 = handle(&mut s, &req("tools/call", 12, Some(params_get))).unwrap();
        let text2 = resp2
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .unwrap()[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(
            text2.contains(script),
            "get_scene must echo back the exact script: {text2}"
        );
        assert!(
            text2.contains("sampling_bounds="),
            "get_scene must include sampling_bounds field (問80): {text2}"
        );
    }

    #[test]
    fn get_scene_default_includes_reproducible_dsl() {
        // 問83: デフォルトシーン状態の get_scene は再現用 DSL スニペットを含む必要がある。
        // AIが run_script をまだ呼んでいない場合でも、デフォルトシーンの DSL を
        // コピーして実行できるようにすることで自己修正ループが成立する。
        let mut s = tools::Session::new();
        let params = json::obj([("name", json::s("get_scene")), ("arguments", json::obj([]))]);
        let resp = handle(&mut s, &req("tools/call", 99, Some(params))).unwrap();
        let text = resp
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .unwrap()[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(
            text.contains("smooth_union"),
            "default get_scene should include 'smooth_union' DSL snippet for reproducibility (問83): {text}"
        );
        assert!(
            text.contains("to reproduce"),
            "default get_scene should include 'to reproduce' hint (問83): {text}"
        );
    }

    #[test]
    fn tools_call_eval_returns_number() {
        // (0.5, 0, 0): inside sphere/cuboid union and outside cylinder hole → SDF < 0
        let mut s = tools::Session::new();
        let val = eval_at(&mut s, 0.5, 0.0, 0.0);
        assert!(
            val < 0.0,
            "SDF at (0.5,0,0) should be inside (negative), got {val}"
        );
    }

    #[test]
    fn run_script_updates_active_scene() {
        // 問12 のリグレッション防止: run_script 後に eval/他ツールが
        // ハードコード形状ではなくスクリプトのシーンを見ることを保証する。
        let mut s = tools::Session::new();
        // 問78: 既定デモは smooth_union(sphere(1), cuboid(0.8)) → 原点は形状内 (負)。
        // 遠点 (10, 0, 0) は外 (正) → 問12 検証に使う。
        let default_far = eval_at(&mut s, 10.0, 0.0, 0.0);
        assert!(default_far > 0.0, "far point in default scene must be outside: {default_far}");

        // 半径 3 の球に差し替える。
        let params = json::obj([
            ("name", json::s("run_script")),
            (
                "arguments",
                json::obj([("script", json::s(r#"{"op":"sphere","r":3.0}"#))]),
            ),
        ]);
        let resp = handle(&mut s, &req("tools/call", 9, Some(params))).unwrap();
        assert_eq!(
            resp.get("result").and_then(|r| r.get("isError")),
            Some(&json::b(false))
        );

        // 原点は半径3球の内部 → SDF ≈ -3。スクリプトが正本になった証拠。
        let v = eval_at(&mut s, 0.0, 0.0, 0.0);
        assert!(
            (v - (-3.0)).abs() < 1e-9,
            "expected -3.0 after run_script, got {v}"
        );
    }

    #[test]
    fn unknown_method_returns_error() {
        let mut s = tools::Session::new();
        let resp = handle(&mut s, &req("unknown/method", 4, None)).unwrap();
        let err = resp
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_f64());
        assert_eq!(err.map(|f| f as i64), Some(-32601));
    }

    #[test]
    fn help_tool_returns_format_reference() {
        // 問37: help ツールが KadoScene 演算子一覧を含む参考文書を返すことを確認する。
        let mut s = tools::Session::new();
        let params = json::obj([
            ("name", json::s("help")),
            ("arguments", json::obj([])),
        ]);
        let resp = handle(&mut s, &req("tools/call", 20, Some(params))).unwrap();
        let text = resp
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .unwrap()[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(text.contains("sphere"), "help must mention sphere");
        assert!(text.contains("smooth_union"), "help must mention smooth_union");
        assert!(text.contains("run_script"), "help must reference workflow");
    }

    #[test]
    fn undo_script_restores_previous_scene() {
        // 問67: run_script で上書きしたシーンを undo_script で1段戻せることを確認。
        let mut s = tools::Session::new();

        // 初期状態の SDF 値を記録 (デフォルトシーン)。
        let initial_val = eval_at(&mut s, 0.0, 0.0, 0.0);

        // 半径 3 の球に差し替える。
        let params_run = json::obj([
            ("name", json::s("run_script")),
            (
                "arguments",
                json::obj([("script", json::s(r#"{"op":"sphere","r":3.0}"#))]),
            ),
        ]);
        handle(&mut s, &req("tools/call", 20, Some(params_run))).unwrap();
        let after_run = eval_at(&mut s, 0.0, 0.0, 0.0);
        assert!(
            (after_run - (-3.0)).abs() < 1e-9,
            "after run_script, sphere r=3 expected: got {after_run}"
        );

        // undo_script: デフォルトシーンへ戻る。
        let params_undo = json::obj([
            ("name", json::s("undo_script")),
            ("arguments", json::obj([])),
        ]);
        let undo_resp = handle(&mut s, &req("tools/call", 21, Some(params_undo.clone()))).unwrap();
        assert_eq!(
            undo_resp.get("result").and_then(|r| r.get("isError")),
            Some(&json::b(false)),
            "first undo must succeed"
        );
        let after_undo = eval_at(&mut s, 0.0, 0.0, 0.0);
        assert!(
            (after_undo - initial_val).abs() < 1e-9,
            "after undo, scene must revert: expected {initial_val}, got {after_undo}"
        );

        // 2回目の undo は履歴なしでエラー。
        let undo2_resp = handle(&mut s, &req("tools/call", 22, Some(params_undo))).unwrap();
        assert_eq!(
            undo2_resp.get("result").and_then(|r| r.get("isError")),
            Some(&json::b(true)),
            "second undo must fail (single-level undo)"
        );
    }

    #[test]
    fn failed_run_script_preserves_undo_state_and_scene() {
        // 問121: run_script が eval 失敗で早期 return するとき、undo 履歴と現在シーンを
        // 破壊してはならない。もし prev_scene の保存が eval 成功の**前**に行われると
        // (もっともらしいリファクタ事故)、失敗した run_script が undo 履歴を現在シーンで
        // 上書きし、直前の成功した変更を取り消せなくなる。この不変条件を固定する。
        //
        // シナリオ:
        //   1. run_script(A=sphere r=3) 成功 → scene=A, undo履歴=default
        //   2. run_script(invalid) 失敗 → scene は A のまま, undo履歴も default のまま
        //   3. undo → A ではなく default (Aの前) に戻る
        let mut s = tools::Session::new();
        let initial_val = eval_at(&mut s, 0.0, 0.0, 0.0);

        // 1. 有効なスクリプト A を適用。
        let run_a = json::obj([
            ("name", json::s("run_script")),
            ("arguments", json::obj([("script", json::s(r#"{"op":"sphere","r":3.0}"#))])),
        ]);
        handle(&mut s, &req("tools/call", 30, Some(run_a))).unwrap();
        let after_a = eval_at(&mut s, 0.0, 0.0, 0.0);
        assert!((after_a - (-3.0)).abs() < 1e-9, "scene A (sphere r=3) applied: {after_a}");

        // 2. 不正なスクリプトを適用 → 失敗するはず。
        let run_bad = json::obj([
            ("name", json::s("run_script")),
            ("arguments", json::obj([("script", json::s(r#"{"op":"not_a_real_op"}"#))])),
        ]);
        let bad_resp = handle(&mut s, &req("tools/call", 31, Some(run_bad))).unwrap();
        assert_eq!(
            bad_resp.get("result").and_then(|r| r.get("isError")),
            Some(&json::b(true)),
            "invalid script must error"
        );
        // 失敗後もシーンは A のまま (破壊されない)。
        let after_bad = eval_at(&mut s, 0.0, 0.0, 0.0);
        assert!(
            (after_bad - (-3.0)).abs() < 1e-9,
            "failed run_script must not change the scene: expected A (-3.0), got {after_bad}"
        );

        // 3. undo → A の前 (default) に戻る。失敗した run が undo 履歴を壊していれば
        //    ここで A (-3.0) のままになり、このアサートが落ちる。
        let undo = json::obj([("name", json::s("undo_script")), ("arguments", json::obj([]))]);
        let undo_resp = handle(&mut s, &req("tools/call", 32, Some(undo))).unwrap();
        assert_eq!(
            undo_resp.get("result").and_then(|r| r.get("isError")),
            Some(&json::b(false)),
            "undo must succeed (history preserved through the failed run)"
        );
        let after_undo = eval_at(&mut s, 0.0, 0.0, 0.0);
        assert!(
            (after_undo - initial_val).abs() < 1e-9,
            "undo must restore to the state before A (default), not to A: expected {initial_val}, got {after_undo}"
        );
    }

    #[test]
    fn run_script_surfaces_multiple_bodies_warning() {
        // 問81: MULTIPLE_BODIES は Warning なので is_ok()=true だが、
        // run_script がサイレントに通過させると AI が切断ボディに気づかない。
        // 2つの離れた球 → MULTIPLE_BODIES warning → run_script 応答に "MULTIPLE_BODIES" が含まれる。
        let mut s = tools::Session::new();
        let two_spheres = r#"{"op":"union",
            "a":{"op":"sphere","r":0.3},
            "b":{"op":"translate","x":5.0,"y":0,"z":0,"shape":{"op":"sphere","r":0.3}}}"#;
        let params = json::obj([
            ("name", json::s("run_script")),
            ("arguments", json::obj([("script", json::s(two_spheres))])),
        ]);
        let resp = handle(&mut s, &req("tools/call", 70, Some(params))).unwrap();
        let text = resp
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .unwrap()[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        // isError=false (スクリプトは有効)。
        assert_eq!(
            resp.get("result").and_then(|r| r.get("isError")),
            Some(&json::b(false)),
            "run_script with disconnected bodies is not an error: {text}"
        );
        // 応答に MULTIPLE_BODIES が含まれる (問81)。
        assert!(
            text.contains("MULTIPLE_BODIES"),
            "run_script must surface MULTIPLE_BODIES warning: {text}"
        );
    }

    #[test]
    fn screenshot_unknown_view_returns_error() {
        // 問71: 未知ビュー名はサイレントフォールバックせずエラーを返す。
        let mut s = tools::Session::new();
        let params = json::obj([
            ("name", json::s("screenshot")),
            (
                "arguments",
                json::obj([("view", json::s("above-45-deg"))]),
            ),
        ]);
        let resp = handle(&mut s, &req("tools/call", 50, Some(params))).unwrap();
        assert_eq!(
            resp.get("result").and_then(|r| r.get("isError")),
            Some(&json::b(true)),
            "unknown view must return isError=true"
        );
        // エラーメッセージに有効なビュー名リストが含まれる。
        let err_text = resp
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .unwrap()[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            err_text.contains("iso") && err_text.contains("front"),
            "error message must list valid views: {err_text}"
        );

        // 既知のビュー名 ("front") は成功する。
        let params_ok = json::obj([
            ("name", json::s("screenshot")),
            (
                "arguments",
                json::obj([("view", json::s("front"))]),
            ),
        ]);
        let resp_ok = handle(&mut s, &req("tools/call", 51, Some(params_ok))).unwrap();
        assert_eq!(
            resp_ok.get("result").and_then(|r| r.get("isError")),
            Some(&json::b(false)),
            "known view 'front' must succeed"
        );
    }

    #[test]
    fn eval_rejects_non_finite_coordinates() {
        // 問69: Infinity/NaN の座標が SDF に渡ると NaN 伝播する。早期拒否することを確認。
        // JSON では 1e999 → Infinity が parse::<f64>() で表現される。
        let mut s = tools::Session::new();

        // x = Infinity
        let params_inf = json::obj([
            ("name", json::s("eval")),
            (
                "arguments",
                json::obj([
                    ("x", json::n(f64::INFINITY)),
                    ("y", json::n(0.0)),
                    ("z", json::n(0.0)),
                ]),
            ),
        ]);
        let resp = handle(&mut s, &req("tools/call", 30, Some(params_inf))).unwrap();
        assert_eq!(
            resp.get("result").and_then(|r| r.get("isError")),
            Some(&json::b(true)),
            "eval with Infinity coordinate must return isError=true"
        );

        // z = NaN
        let params_nan = json::obj([
            ("name", json::s("eval")),
            (
                "arguments",
                json::obj([
                    ("x", json::n(0.0)),
                    ("y", json::n(0.0)),
                    ("z", json::n(f64::NAN)),
                ]),
            ),
        ]);
        let resp2 = handle(&mut s, &req("tools/call", 31, Some(params_nan))).unwrap();
        assert_eq!(
            resp2.get("result").and_then(|r| r.get("isError")),
            Some(&json::b(true)),
            "eval with NaN coordinate must return isError=true"
        );

        // 有限値は通常通り成功。
        let params_ok = json::obj([
            ("name", json::s("eval")),
            (
                "arguments",
                json::obj([
                    ("x", json::n(0.5)),
                    ("y", json::n(0.0)),
                    ("z", json::n(0.0)),
                ]),
            ),
        ]);
        let resp3 = handle(&mut s, &req("tools/call", 32, Some(params_ok))).unwrap();
        assert_eq!(
            resp3.get("result").and_then(|r| r.get("isError")),
            Some(&json::b(false)),
            "eval with finite coordinates must succeed"
        );
    }

    #[test]
    fn get_scene_reports_undo_availability() {
        // 問74: get_scene は undo_script が使えるかどうかを undo_available フィールドで報告する。
        // AI が盲目的に undo を試みる前に状態を確認できる。
        let mut s = tools::Session::new();
        let params_get = json::obj([("name", json::s("get_scene")), ("arguments", json::obj([]))]);

        // 初期状態: undo 不可。
        let resp1 = handle(&mut s, &req("tools/call", 60, Some(params_get.clone()))).unwrap();
        let text1 = resp1
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .unwrap()[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(
            text1.contains("undo_available=false"),
            "before any run_script, undo must not be available: {text1}"
        );

        // run_script でシーンを変更 → undo 可能になる。
        let params_run = json::obj([
            ("name", json::s("run_script")),
            ("arguments", json::obj([("script", json::s(r#"{"op":"sphere","r":1.0}"#))])),
        ]);
        handle(&mut s, &req("tools/call", 61, Some(params_run))).unwrap();

        let resp2 = handle(&mut s, &req("tools/call", 62, Some(params_get.clone()))).unwrap();
        let text2 = resp2
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .unwrap()[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(
            text2.contains("undo_available=true"),
            "after run_script, undo must be available: {text2}"
        );

        // undo_script を呼ぶ → 再び undo 不可。
        let params_undo = json::obj([("name", json::s("undo_script")), ("arguments", json::obj([]))]);
        handle(&mut s, &req("tools/call", 63, Some(params_undo))).unwrap();

        let resp3 = handle(&mut s, &req("tools/call", 64, Some(params_get))).unwrap();
        let text3 = resp3
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .unwrap()[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(
            text3.contains("undo_available=false"),
            "after undo_script, undo must not be available again: {text3}"
        );
    }

    #[test]
    fn build_dir_array_z_element_defaults_to_zero_not_one() {
        // 問85: [dx, dy, dz] 配列の z 要素が non-numeric のとき旧コードは 1.0 でフォールバックし、
        // ユーザーが [1,0,0] を意図しても [1,0,1] (対角) で解析される誤りがあった。
        // 修正後は unwrap_or(0.0) → 欠損 z は 0 扱い。
        // validate ツールに [1,0,0] の build_dir を渡して、問題なく受け付けることを確認。
        let mut s = tools::Session::new();
        let script = r#"{"op":"sphere","r":1.0}"#;
        let params_run = json::obj([
            ("name", json::s("run_script")),
            ("arguments", json::obj([("script", json::s(script))])),
        ]);
        handle(&mut s, &req("tools/call", 90, Some(params_run))).unwrap();

        let params_val = json::obj([
            ("name", json::s("validate")),
            (
                "arguments",
                json::obj([
                    ("build_dir", json::arr([json::n(1.0), json::n(0.0), json::n(0.0)])),
                ]),
            ),
        ]);
        let resp = handle(&mut s, &req("tools/call", 91, Some(params_val))).unwrap();
        let is_error = resp
            .get("result")
            .and_then(|r| r.get("isError"))
            .and_then(|v| v.as_bool());
        assert_eq!(
            is_error,
            Some(false),
            "validate with explicit [1,0,0] build_dir must succeed"
        );
    }

    #[test]
    fn notification_returns_none() {
        let mut s = tools::Session::new();
        let notif = json::obj([
            ("jsonrpc", json::s("2.0")),
            ("method", json::s("initialized")),
        ]);
        assert!(handle(&mut s, &notif).is_none());
    }

    #[test]
    fn malformed_tools_call_returns_uniform_error_shape() {
        // 問106: 不正な tools/call (name 欠落、name 非文字列、arguments 非オブジェクト) は
        // パニックせず、未知ツールと同じ {content, isError:true} 形のツール結果を返す。
        let mut s = tools::Session::new();

        // ツール結果 (result.content/isError) を取り出すヘルパ。
        let call = |s: &mut tools::Session, params: Value| -> (bool, String) {
            let resp = handle(s, &req("tools/call", 1, Some(params))).unwrap();
            let result = resp.get("result").expect("must have result");
            let is_err = result.get("isError").and_then(|v| v.as_bool()).expect("isError present");
            let text = result
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
                .and_then(|c| c.get("text"))
                .and_then(|v| v.as_str())
                .expect("content[0].text present")
                .to_string();
            (is_err, text)
        };

        // name 欠落 → isError:true, content あり (code/message の裸オブジェクトではない)。
        let (e1, _) = call(&mut s, json::obj([("arguments", json::obj([]))]));
        assert!(e1, "missing name must yield isError:true");

        // name が非文字列 (数値) → 同上。
        let (e2, _) = call(
            &mut s,
            json::obj([("name", json::n(42.0)), ("arguments", json::obj([]))]),
        );
        assert!(e2, "non-string name must yield isError:true");

        // arguments が非オブジェクト (文字列) でもパニックしない。
        // eval は引数を読めず arg エラーになるが unknown tool ではない。
        let (e3, t3) = call(
            &mut s,
            json::obj([("name", json::s("eval")), ("arguments", json::s("not-an-object"))]),
        );
        assert!(e3, "eval with non-object arguments must error gracefully");
        assert!(!t3.contains("unknown tool"), "must dispatch eval, not 'unknown tool': {t3}");

        // 未知ツール名 → isError:true で "unknown tool" を含む (既存の整合した経路)。
        let (e4, t4) = call(
            &mut s,
            json::obj([("name", json::s("nonexistent_tool")), ("arguments", json::obj([]))]),
        );
        assert!(e4 && t4.contains("unknown tool"), "unknown tool path unchanged: {t4}");
    }

    #[test]
    fn oversized_content_length_is_rejected_before_allocation() {
        // 問118: 巨大 Content-Length は確保前に拒否される (OOM/DoS 防御)。
        // 信頼できないクライアントが Content-Length: 999999999999 を送っても
        // テラバイト級の vec を確保せず、即座に InvalidData エラーを返す。
        let header = format!("Content-Length: {}\r\n\r\n", MAX_MESSAGE_BYTES + 1);
        let mut cursor = std::io::Cursor::new(header.into_bytes());
        let err = read_message(&mut cursor).expect_err("oversized frame must be rejected");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(
            err.to_string().contains("exceeds limit"),
            "error must explain the limit: {err}"
        );

        // 上限ちょうどの宣言は許容される (本文が足りなければ EOF になるが、
        // 確保前の上限チェックは通過する = 拒否されない)。
        let header_ok = format!("Content-Length: {MAX_MESSAGE_BYTES}\r\n\r\n");
        let mut cursor_ok = std::io::Cursor::new(header_ok.into_bytes());
        let err_ok = read_message(&mut cursor_ok).expect_err("body is absent so read_exact fails");
        // 上限超過エラーではなく、本文不足 (EOF) であることを確認。
        assert!(
            !err_ok.to_string().contains("exceeds limit"),
            "at-limit declaration must pass the size gate, got: {err_ok}"
        );

        // 通常サイズのフレームは正常にパースされる (回帰: 正当な経路を壊さない)。
        let body = r#"{"jsonrpc":"2.0","method":"ping","id":1}"#;
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let mut cursor_good = std::io::Cursor::new(frame.into_bytes());
        let msg = read_message(&mut cursor_good).expect("normal frame must parse");
        assert_eq!(msg.get("method").and_then(|m| m.as_str()), Some("ping"));
    }

    #[test]
    fn missing_or_non_numeric_content_length_is_rejected_identically() {
        // 問171/172: Content-Length が無い、または非数値の場合、どちらも
        // "missing Content-Length" として拒否される。非数値は parse().ok() が
        // None になり content_length が未設定のまま = 欠落と同じ扱いになる。
        // (オーバーフローや巨大確保の前に弾かれる)。

        // (1) ヘッダ自体が無い (別ヘッダのみ)。
        let no_cl = "Other-Header: value\r\n\r\n";
        let mut cursor = std::io::Cursor::new(no_cl.as_bytes().to_vec());
        let err = read_message(&mut cursor).expect_err("missing Content-Length must be rejected");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(
            err.to_string().contains("missing Content-Length"),
            "missing header error must say so, got: {err}"
        );

        // (2) Content-Length が非数値 → parse().ok() = None → 欠落と同一エラー。
        let bad_cl = "Content-Length: notanumber\r\n\r\n";
        let mut cursor2 = std::io::Cursor::new(bad_cl.as_bytes().to_vec());
        let err2 = read_message(&mut cursor2).expect_err("non-numeric Content-Length must be rejected");
        assert_eq!(err2.kind(), io::ErrorKind::InvalidData);
        assert!(
            err2.to_string().contains("missing Content-Length"),
            "non-numeric value is treated as missing, got: {err2}"
        );
    }
}
