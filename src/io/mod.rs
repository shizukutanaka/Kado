//! 出力フォーマット (Plan §3: STL / 3MF / GLB / HTMLビューア)。
//!
//! 製造の最小共通項 **binary STL**、インデックス付き・閲覧容易な
//! **GLB (glTF 2.0 binary)**、現代的プリント標準 **3MF** (単位付き OPC/ZIP)、
//! オフライン閲覧用の **自己完結 HTML ビューア** (WebGL2) を実装する
//! (facetted STEP は問7で BACKLOG 降格)。

pub mod gltf;
pub mod html;
pub mod stl;
pub mod threemf;
pub mod zip;
