// デスクトップ版：Web版と同じ画面（同じWASM）をWebViewの中で動かすだけ。
// 計算コア（crates/core）はここからは呼ばない（Web版と挙動が分かれないように）。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(e) = tauri::Builder::default().run(tauri::generate_context!()) {
        eprintln!("起動に失敗しました: {e}");
    }
}
