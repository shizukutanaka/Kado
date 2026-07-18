//! MCP エンドツーエンド統合テスト (問293)。
//!
//! 問292 で、`chamfer_*` が JSON 評価器では動くのにテキスト DSL では
//! "unknown function" になる回帰が、**実 MCP バイナリでの通し検証**で初めて
//! 発覚した。ユニットテストは各層を個別に検証していたが、AI が実際に叩く経路
//! (MCP stdio + JSON-RPC + テキスト DSL) を通していなかったためである。
//!
//! このテストは実際に `kado mcp` バイナリを子プロセスとして起動し、
//! Content-Length フレーミングの JSON-RPC を stdin へ流し、stdout の応答を
//! パースして、AI ワークフロー全体 (initialize → run_script → validate →
//! screenshot) が機能することを固定する。ユニットテストでは捕まえられない
//! 「表面積の不整合」を一点で検知する恒久ガード。

use std::io::{Read, Write};
use std::process::{Command, Stdio};

use kado::mcp::json::{self, Value};

/// JSON-RPC メッセージを Content-Length フレーミングでエンコードする。
fn frame(body: &str) -> Vec<u8> {
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(body.as_bytes());
    out
}

/// `kado mcp` を起動し、与えたリクエスト群を送って stdout 全体を返す。
fn run_mcp(requests: &[String]) -> Vec<u8> {
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

/// Content-Length フレーム列を id→結果 Value へパースする。
fn parse_responses(bytes: &[u8]) -> std::collections::BTreeMap<i64, Value> {
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

#[test]
fn full_ai_workflow_over_real_mcp_stdio() {
    // AI が実際に辿る経路: initialize → chamfer をテキスト DSL で run_script →
    // validate → screenshot。問285〜292 の機能を一気通貫で運動させる。
    let reqs = vec![
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#.to_string(),
        // 問285/292: chamfer_union を**テキスト DSL**で。ここが 問292 の回帰点。
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"run_script","arguments":{"script":"chamfer_union(cuboid(1.0), sphere(1.2), 0.3)"}}}"#.to_string(),
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"validate","arguments":{"resolution":40}}}"#.to_string(),
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"screenshot","arguments":{"width":48,"height":48,"resolution":24}}}"#.to_string(),
    ];
    let out = run_mcp(&reqs);
    let resp = parse_responses(&out);

    // 1) initialize は最新安定版 2025-11-25 を返す (問286)。
    let init = resp.get(&1).expect("initialize response");
    assert_eq!(
        init.get("result")
            .and_then(|r| r.get("protocolVersion"))
            .and_then(|v| v.as_str()),
        Some("2025-11-25"),
        "initialize must negotiate 2025-11-25"
    );

    // 2) chamfer_union をテキスト DSL で run_script — 問292 の回帰点。isError=false。
    let run = resp.get(&2).expect("run_script response");
    let run_err = run
        .get("result")
        .and_then(|r| r.get("isError"))
        .and_then(|v| v.as_bool());
    assert_eq!(
        run_err,
        Some(false),
        "chamfer_union via text DSL must succeed (問292 regression guard); \
         response: {:?}",
        run.get("result")
    );

    // 3) validate は manifold なメッシュを報告する (chamfer 結果が水密)。
    let val = resp.get(&3).expect("validate response");
    let text = val
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("text"))
        .and_then(|v| v.as_str())
        .expect("validate text payload");
    let report = json::parse(text).expect("validate report is JSON");
    assert_eq!(
        report.get("manifold").and_then(|v| v.as_bool()),
        Some(true),
        "chamfer_union mesh must be manifold"
    );
    assert!(
        report
            .get("triangles")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            > 0.0,
        "validate must report a non-empty mesh"
    );

    // 4) screenshot は image と目盛り凡例 text の両方を返す (問288)。
    let shot = resp.get(&4).expect("screenshot response");
    let content = shot
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .expect("screenshot content array");
    let has_image = content.iter().any(|c| {
        c.get("type").and_then(|v| v.as_str()) == Some("image")
            && c.get("mimeType").and_then(|v| v.as_str()) == Some("image/png")
    });
    assert!(has_image, "screenshot must return a PNG image");
    let note = content
        .iter()
        .find_map(|c| c.get("text").and_then(|v| v.as_str()))
        .unwrap_or("");
    assert!(
        note.contains("Tick marks") && note.contains("mm"),
        "screenshot response must carry the mm tick-spacing note (問288), got: {note}"
    );
}

#[test]
fn invalid_tool_input_is_tool_execution_error_over_stdio() {
    // 問286: 2025-11-25 は入力検証エラーを Protocol Error でなく Tool Execution
    // Error (isError:true) で返すことを求める。実バイナリでも JSON-RPC error では
    // なく isError:true になることを固定。
    let reqs = vec![
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#.to_string(),
        // 負の半径は評価前に拒否される (SPEC の事前検証)。
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"run_script","arguments":{"script":"sphere(-1.0)"}}}"#.to_string(),
    ];
    let resp = parse_responses(&run_mcp(&reqs));
    let r = resp.get(&2).expect("run_script response");
    assert!(
        r.get("error").is_none(),
        "input validation must not surface as a JSON-RPC protocol error"
    );
    assert_eq!(
        r.get("result")
            .and_then(|x| x.get("isError"))
            .and_then(|v| v.as_bool()),
        Some(true),
        "invalid tool input must be a Tool Execution Error (isError:true)"
    );
}
