//! MCP サーバー (Phase 4 / Phase 0.5スパイク)。
//!
//! プロトコル: MCP 2024-11-05 / JSON-RPC 2.0 / stdio transport
//! (Content-Length フレーミング、LSP スタイル)。
//! ツール数 ≤ 12 (Plan §3)。read-only 既定 (Plan §3 / C9)。
//!
//! Phase 0.5 実装ツール:
//! - `screenshot`  SDF 形状を PNG にレンダリングし base64 返却
//! - `export`      STL ファイルを生成しパスを返却
//! - `eval`        SDF 点評価 (最小スクリプト検証用)

pub mod json;
pub mod server;
pub mod tools;

pub use server::run_stdio;
