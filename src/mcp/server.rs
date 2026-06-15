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

/// stdio で MCP サーバーを起動する。返らない。
pub fn run_stdio() -> ! {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    // セッション状態 (正本シーン)。run_script で更新され他ツールが参照する。
    let mut session = tools::Session::new();

    loop {
        match read_message(&mut reader) {
            Ok(msg) => {
                if let Some(resp) = handle(&mut session, &msg) {
                    if write_message(&mut writer, &resp).is_err() {
                        break;
                    }
                }
            }
            Err(_) => break, // stdin EOF または不正フレーム
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
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return rpc_error(-32602, "missing tool name"),
    };
    let empty = json::obj([]);
    let args = params.get("arguments").unwrap_or(&empty);

    let result = tools::call_tool(session, &name, args);
    json::obj([
        ("content", Value::Array(result.content)),
        ("isError", json::b(result.is_error)),
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

fn rpc_error(code: i64, message: &str) -> Value {
    json::obj([
        ("code", json::n(code as f64)),
        ("message", json::s(message)),
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
            text2.contains("bounds="),
            "get_scene must include sampling bounds: {text2}"
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
        // 既定 (デモ) では原点は穴の中 → 正。
        assert!(eval_at(&mut s, 0.0, 0.0, 0.0) > 0.0);

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
    fn notification_returns_none() {
        let mut s = tools::Session::new();
        let notif = json::obj([
            ("jsonrpc", json::s("2.0")),
            ("method", json::s("initialized")),
        ]);
        assert!(handle(&mut s, &notif).is_none());
    }
}
