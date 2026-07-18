//! MCP サーバー。
//!
//! プロトコル: MCP (2025-11-25 / 2025-06-18 / 2024-11-05 を版交渉・問286) /
//! JSON-RPC 2.0 / stdio transport (Content-Length フレーミング、LSP スタイル)。
//! read-only 既定 (Plan §3 / C9)。
//!
//! 公開ツール (8): `run_script` `eval` `validate` `screenshot` `export`
//! `get_scene` `undo_script` `help`。エンドツーエンドの経路 (stdio + JSON-RPC +
//! テキスト DSL) は `tests/mcp_workflow.rs` が実バイナリで通しテストする (問293)。

pub mod json;
pub mod server;
pub mod tools;

pub use server::run_stdio;
