//! 出力フォーマット (Plan §3: STL / 3MF / GLB / HTMLビューア)。
//!
//! 製造の最小共通項 **binary STL**、インデックス付き・閲覧容易な
//! **GLB (glTF 2.0 binary)**、現代的プリント標準 **3MF** (単位付き OPC/ZIP) を
//! 実装する。HTML ビューアは今後追加 (facetted STEP は問7で BACKLOG 降格)。

pub mod gltf;
pub mod stl;
pub mod threemf;
pub mod zip;
