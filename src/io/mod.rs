//! 出力フォーマット (Plan §3: STL / 3MF / GLB / HTMLビューア)。
//!
//! まず製造の最小共通項である **binary STL** を実装する。3MF/GLB/HTMLは
//! Phase 5 で追加 (facetted STEP は問7で BACKLOG 降格)。

pub mod stl;
