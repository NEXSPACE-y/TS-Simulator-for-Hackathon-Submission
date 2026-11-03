//! TS実習シミュレーターの計算コア。
//!
//! 状態・物理・判定・観測の計算・タイマーをすべてここに置く（SPEC 2章）。
//! WASM・Tauri・ブラウザには依存しない純粋なRust。panicさせない。

pub mod angle;
pub mod coach;
pub mod config;
pub mod instrument;
pub mod math;
pub mod rng;
pub mod sim;
pub mod steps;
pub mod survey;
pub mod tripod;

pub use instrument::Thumb;
pub use sim::{Command, Mode, Phase, Simulator, Snapshot, Speed, UiState};
