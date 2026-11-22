// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown payload type".to_string()
        };
        let _ = std::fs::write(
            "C:\\Users\\shaur\\NeoResonance\\panic_detail.txt",
            format!("PANIC: {}\nLocation: {:?}", payload, info.location()),
        );
    }));

    neo_resonance_lib::run();
}
