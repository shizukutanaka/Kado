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

mod common;

use common::{parse_responses, run_mcp, tool_ok, tool_text};
use kado::mcp::json;

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

/// Plan.md の**旗艦 DoD** を実行可能なテストにする (問309)。
///
/// Plan.md §4 Phase 4 の DoD はこう書かれている:
///   「**M3穴付きブラケット**を自然言語→検証済み STL まで無人完走」
/// また §7 の KPI は「**平均ツール呼出 ≤15/タスク**」を課している。
///
/// ところがこの2つは**一度も計測されていなかった** — 製品が自らの合格条件を
/// 検証していない状態だった (「計測しない要件は要件ではなく願望である」)。
/// 本テストは AI が実際に辿る MCP 経路だけで DoD 全体を完走し、
/// **消費したツール呼出数を数えて KPI と突き合わせる**。
#[test]
fn flagship_dod_m3_bracket_completes_within_the_tool_call_budget() {
    // KPI (Plan.md §7): 平均ツール呼出 ≤15/タスク。
    const TOOL_CALL_BUDGET: usize = 15;

    // AI が「M3穴付きブラケット」を無人で作り切る最小の道具列。
    // initialize はプロトコル握手でありツール呼出ではないので予算に数えない。
    let out_stl = "kado-dod-bracket.stl";
    let tool_calls: Vec<String> = vec![
        // 1. 形状を作る: 40x40x4mm の板に M3 クリアランス穴 (Ø3.2 = r1.6) を貫通させる。
        format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"run_script","arguments":{{"script":"difference(cuboid(20.0,20.0,2.0), cylinder(1.6,10.0))"}}}}}}"#
        ),
        // 2. 穴径を実測して意図どおりか確認する (問299 の measure が無ければ約30呼出を要した)。
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"measure","arguments":{"from":[-50.0,0.0,0.0],"dir":[1.0,0.0,0.0]}}}"#.to_string(),
        // 3. 製造可能性を検証する。
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"validate","arguments":{"min_wall_mm":1.0}}}"#.to_string(),
        // 4. 出荷する。
        format!(
            r#"{{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{{"name":"export","arguments":{{"path":"{out_stl}"}}}}}}"#
        ),
    ];
    assert!(
        tool_calls.len() <= TOOL_CALL_BUDGET,
        "the DoD workflow uses {} tool calls, over the KPI budget of {TOOL_CALL_BUDGET}",
        tool_calls.len()
    );

    let mut reqs =
        vec![r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#.to_string()];
    reqs.extend(tool_calls.iter().cloned());
    let resp = parse_responses(&run_mcp(&reqs));

    // 全ツール呼出が成功すること (無人完走 = 途中で人手の介入を要さない)。
    for id in 2..=5 {
        let r = resp
            .get(&id)
            .unwrap_or_else(|| panic!("no response for tool call id={id}"));
        assert_eq!(
            r.get("result")
                .and_then(|x| x.get("isError"))
                .and_then(|v| v.as_bool()),
            Some(false),
            "tool call id={id} failed, so the run was not unattended: {:?}",
            r.get("result")
        );
    }

    // 2. measure: 穴が M3 クリアランス Ø3.2 であることを**数値で**確認する。
    let m_text = resp[&3]
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("text"))
        .and_then(|v| v.as_str())
        .expect("measure text payload");
    let m = json::parse(m_text).expect("measure returns JSON");
    assert_eq!(
        m.get("complete").and_then(|v| v.as_bool()),
        Some(true),
        "the measuring ray must complete, or the dimension is unverified (問301)"
    );
    let spans = m
        .get("spans")
        .and_then(|v| v.as_array())
        .expect("measure returns spans");
    // 材料 → 穴 → 材料 なので中央の span が穴径。
    assert_eq!(spans.len(), 3, "solid→hole→solid must yield 3 spans");
    let hole_dia = spans[1].as_f64().expect("hole span is numeric");
    assert!(
        (hole_dia - 3.2).abs() < 1e-3,
        "the M3 clearance hole must measure Ø3.2mm, got {hole_dia}"
    );

    // 3. validate: 製造可能性の判定が下せること (水密であることは必須)。
    let v_text = resp[&4]
        .get("result")
        .and_then(|r| r.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("text"))
        .and_then(|v| v.as_str())
        .expect("validate text payload");
    let report = json::parse(v_text).expect("validate returns JSON");
    assert_eq!(
        report.get("manifold").and_then(|v| v.as_bool()),
        Some(true),
        "the bracket must be watertight to be manufacturable"
    );

    // 4. export: 検証済み STL が実在し、binary STL として妥当であること
    //    (「検証済み STL まで」の "STL" を実ファイルで確認する)。
    let path = std::path::Path::new(out_stl);
    let bytes = std::fs::read(path).expect("the DoD requires an actual STL file on disk");
    std::fs::remove_file(path).ok();
    let decoded =
        kado::io::stl::decode_binary(&bytes).expect("the exported file must be a valid binary STL");
    assert!(
        decoded.is_edge_manifold(),
        "the shipped STL must itself be watertight"
    );
}

/// 問318: 引数のサイレントフォールバックが**実バイナリ経由**で消えたことを確認する。
///
/// ユニットテストは `arg_*` を直接叩くが、AI が実際に辿るのは stdio 経由の
/// `tools/call` である。CLAUDE.md §3「完了の定義」に従い、その経路で
/// `isError:true` と**理由の分かるメッセージ**が返ることを固定する。
///
/// 最も重いのは `build_dir` の取り違えである。従来は未知の値を黙って +Z へ倒して
/// いたため、AI が `"up"` や `[1,0]` を渡すと **+Z 前提のオーバーハング判定**が
/// 「頼んだ向きの判定結果」として返り、誤った製造可否を信じることになった。
#[test]
fn malformed_arguments_are_rejected_with_a_reason_over_stdio() {
    let mut reqs = vec![
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#.to_string(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"run_script","arguments":{"script":"sphere(1.0)"}}}"#.to_string(),
    ];

    // (id, tools/call arguments, エラーメッセージに必ず現れる語)
    // 個数はスライスから導く (問319 と同じ: 二重に持てば片方だけずれる)。
    let cases: &[(i64, &str, &str)] = &[
        (
            10,
            r#""name":"validate","arguments":{"build_dir":"up"}"#,
            "build_dir",
        ),
        (
            11,
            r#""name":"validate","arguments":{"build_dir":[1,0]}"#,
            "3",
        ),
        (
            12,
            r#""name":"validate","arguments":{"build_dir":[0,0,0]}"#,
            "overhang",
        ),
        (
            13,
            r#""name":"validate","arguments":{"resolution":100000}"#,
            "resolution",
        ),
        (
            14,
            r#""name":"screenshot","arguments":{"width":0}"#,
            "width",
        ),
        // 問326: 型違いの閾値・文字列・真偽値。以前はすべて黙って既定値になり、
        // AI は「自分が指定した閾値で検証された」と信じた別の閾値の合否を受け取った。
        (
            15,
            r#""name":"validate","arguments":{"min_wall_mm":"0.8"}"#,
            "min_wall_mm",
        ),
        (
            16,
            r#""name":"validate","arguments":{"max_overhang_deg":true}"#,
            "max_overhang_deg",
        ),
        (17, r#""name":"screenshot","arguments":{"view":5}"#, "view"),
        (
            18,
            r#""name":"screenshot","arguments":{"axes":"no"}"#,
            "axes",
        ),
        (19, r#""name":"export","arguments":{"path":123}"#, "path"),
    ];
    for &(id, args, _) in cases {
        reqs.push(format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{{args}}}}}"#
        ));
    }
    // 問326: 負値は verify/check.rs が「0 以下でスキップ」を契約として文書化しており、
    // 誤りではなく指示である。厳格化の勢いで契約を上書きしていないことを固定する。
    reqs.push(
        r#"{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"validate","arguments":{"min_wall_mm":-1,"max_overhang_deg":0}}}"#
            .to_string(),
    );
    // 正常な値は従来どおり通る (回帰: 厳格化で正常経路を壊していないこと)。
    reqs.push(
        r#"{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"validate","arguments":{"build_dir":"-z","resolution":24}}}"#
            .to_string(),
    );

    let resp = parse_responses(&run_mcp(&reqs));

    for &(id, args, needle) in cases {
        let r = resp
            .get(&id)
            .unwrap_or_else(|| panic!("no response for id {id} ({args})"));
        assert!(
            r.get("error").is_none(),
            "id {id}: argument validation must be a Tool Execution Error, not a protocol error"
        );
        assert!(
            !tool_ok(r),
            "id {id}: malformed argument must set isError:true — silently falling back to a \
             default makes the AI trust a result it did not ask for (問318). args: {args}"
        );
        let text = tool_text(r).unwrap_or("");
        assert!(
            text.contains(needle),
            "id {id}: error must explain what was wrong (expected to mention '{needle}'): {text}"
        );
    }

    let neg = resp.get(&21).expect("negative threshold response");
    assert!(
        tool_ok(neg),
        "negative thresholds mean 'skip' per verify/check.rs and must stay accepted (問326): {:?}",
        tool_text(neg)
    );
    let ok = resp.get(&20).expect("valid arguments response");
    assert!(
        tool_ok(ok),
        "valid build_dir/resolution must still succeed: {:?}",
        tool_text(ok)
    );
}
