// 各統合テストは**別クレート**としてコンパイルされ、`tests/common/` はその都度
// 個別にビルドされる。よって「eval_set.rs だけが使うヘルパ」は mcp_workflow.rs の
// コンパイル単位からは未使用に見え、dead_code 警告になる (CI は RUSTFLAGS="-D warnings"
// なので警告=失敗)。共有テストモジュールの定石どおり、モジュール単位で許可する。
#![allow(dead_code)]

//! 統合テスト共通の MCP ハーネス (問311)。
//!
//! 実 `kado mcp` バイナリを子プロセスとして起動し、Content-Length フレーミングの
//! JSON-RPC を stdin へ流して stdout を解釈する。`mcp_workflow.rs` (問293) が持って
//! いたものを、`eval_set.rs` からも使えるよう共通モジュールへ移した——統合テストは
//! 別クレートなので、共有にはこの `tests/common/` 方式が必要になる。

use std::io::{Read, Write};
use std::process::{Command, Stdio};

pub use kado::mcp::json::Value;
use kado::mcp::json::{self};

/// JSON-RPC メッセージを Content-Length フレーミングでエンコードする。
fn frame(body: &str) -> Vec<u8> {
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(body.as_bytes());
    out
}

/// `kado mcp` を起動し、与えたリクエスト群を送って stdout 全体を返す。
pub fn run_mcp(requests: &[String]) -> Vec<u8> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kado"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn kado mcp");

    {
        let mut stdin = child.stdin.take().unwrap();
        for r in requests {
            stdin.write_all(&frame(r)).unwrap();
        }
        // stdin を drop すると EOF となりサーバは正常終了する。
    }

    let mut out = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut out)
        .expect("read stdout");
    child.wait().expect("wait");
    out
}

/// Content-Length フレーム列を id→応答 Value へパースする。
pub fn parse_responses(bytes: &[u8]) -> std::collections::BTreeMap<i64, Value> {
    let mut map = std::collections::BTreeMap::new();
    let mut i = 0;
    let needle = b"Content-Length:";
    while let Some(rel) = find(&bytes[i..], needle) {
        let hdr_start = i + rel;
        let sep = find(&bytes[hdr_start..], b"\r\n\r\n").expect("frame header terminator");
        let len_str = std::str::from_utf8(&bytes[hdr_start + needle.len()..hdr_start + sep])
            .unwrap()
            .trim();
        let len: usize = len_str.parse().expect("content-length value");
        let body_start = hdr_start + sep + 4;
        let body = &bytes[body_start..body_start + len];
        let doc = json::parse(std::str::from_utf8(body).unwrap()).expect("valid JSON response");
        if let Some(id) = doc.get("id").and_then(|v| v.as_f64()) {
            map.insert(id as i64, doc);
        }
        i = body_start + len;
    }
    map
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// ツール結果の最初の text ペイロードを取り出す。
pub fn tool_text(resp: &Value) -> Option<&str> {
    resp.get("result")?
        .get("content")?
        .as_array()?
        .first()?
        .get("text")?
        .as_str()
}

/// ツール呼び出しが成功したか (`isError:false`)。
pub fn tool_ok(resp: &Value) -> bool {
    resp.get("result")
        .and_then(|r| r.get("isError"))
        .and_then(|v| v.as_bool())
        == Some(false)
}
