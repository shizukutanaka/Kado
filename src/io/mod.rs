//! 出力フォーマット (Plan §3: STL / 3MF / GLB / HTMLビューア)。
//!
//! 製造の最小共通項 **binary STL** と、インデックス付き・閲覧容易な
//! **GLB (glTF 2.0 binary)** を実装する。3MF/HTML は今後追加
//! (facetted STEP は問7で BACKLOG 降格)。

pub mod gltf;
pub mod stl;
