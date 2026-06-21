#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(feature = "tauri-commands")]
    stelo_ptc_lib::run();

    #[cfg(not(feature = "tauri-commands"))]
    eprintln!("Built without tauri-commands feature — desktop UI unavailable");
}
