//! メッシュ抽出 (Phase 1 / Phase 0.5スパイク基盤)。
//!
//! SDF木をサンプリングして三角形メッシュを得る。Plan の最終目標は特徴保持
//! **manifold dual contouring** だが (問11)、まず最短でスパイクを通すため
//! 破綻しにくい **marching tetrahedra** を暫定実装する。MTは曖昧ケースが無く、
//! 隣接セル間でエッジ補間を正準化することで**水密**メッシュを保証する。
//!
//! DC への置換は Phase 1 本体で行う (鋭利エッジ保持・面数削減のため)。

pub mod marching_tetrahedra;
pub mod mesh;

pub use marching_tetrahedra::polygonize;
pub use mesh::Mesh;
