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

    loop {
        match read_message(&mut reader) {
            Ok(msg) => {
                if let Some(resp) = handle(&msg) {
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

fn handle(msg: &Value) -> Option<Value> {
    let method = msg.get("method")?.as_str()?;
    let id = msg.get("id").cloned().unwrap_or(json::NULL);
    let params = msg.get("params").cloned().unwrap_or(json::NULL);

    // notifications (id なし) はレスポンス不要。
    let is_notification = msg.get("id").is_none();

    let result = match method {
        "initialize" => Some(handle_initialize(&params)),
        "initialized" => return None, // notification
        "tools/list" => Some(handle_tools_list()),
        "tools/call" => Some(handle_tools_call(&params)),
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

fn handle_tools_call(params: &Value) -> Value {
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return rpc_error(-32602, "missing tool name"),
    };
    let empty = json::obj([]);
    let args = params.get("arguments").unwrap_or(&empty);

    let result = tools::call_tool(&name, args);
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

    #[test]
    fn initialize_returns_protocol_version() {
        let resp = handle(&req("initialize", 1, None)).unwrap();
        let ver = resp
            .get("result")
            .and_then(|r| r.get("protocolVersion"))
            .and_then(|v| v.as_str());
        assert_eq!(ver, Some(MCP_PROTOCOL_VERSION));
    }

    #[test]
    fn tools_list_has_three_tools() {
        let resp = handle(&req("tools/list", 2, None)).unwrap();
        let tools = resp
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(|v| v.as_array());
        assert_eq!(tools.map(|a| a.len()), Some(3));
    }

    #[test]
    fn tools_call_eval_returns_number() {
        // (0.5, 0, 0): inside sphere/cuboid union and outside cylinder hole → SDF < 0
        let params = json::obj([
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
        let resp = handle(&req("tools/call", 3, Some(params))).unwrap();
        let content = resp
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array());
        assert!(content.is_some());
        let text = content.unwrap()[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap();
        let val: f64 = text.trim().parse().unwrap();
        assert!(
            val < 0.0,
            "SDF at (0.5,0,0) should be inside (negative), got {val}"
        );
    }

    #[test]
    fn unknown_method_returns_error() {
        let resp = handle(&req("unknown/method", 4, None)).unwrap();
        let err = resp
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|c| c.as_f64());
        assert_eq!(err.map(|f| f as i64), Some(-32601));
    }

    #[test]
    fn notification_returns_none() {
        let notif = json::obj([
            ("jsonrpc", json::s("2.0")),
            ("method", json::s("initialized")),
        ]);
        assert!(handle(&notif).is_none());
    }
}
