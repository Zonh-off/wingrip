#![windows_subsystem = "windows"]

pub mod config;
pub mod input;
pub mod logic;
pub mod settings;
pub mod ui;

use crossbeam_channel::unbounded;
use std::thread;

fn main() {
    println!("wingrip background service starting...");

    // Start background Settings GUI panel and System Tray Icon thread
    settings::spawn_settings_thread();

    // Channels for communication
    // Input Thread -> Logic Thread
    let (input_tx, input_rx) = unbounded::<input::InputEvent>();

    // Logic Thread -> UI Overlay Thread
    let (ui_tx, ui_rx) = unbounded::<ui::UiEvent>();

    // Start UI Overlay Thread
    let ui_handle = thread::spawn(move || {
        if let Err(e) = ui::run_ui_loop(ui_rx) {
            eprintln!("Error in UI Overlay Thread: {:?}", e);
        }
    });

    // Start Logic Thread
    let logic_handle = thread::spawn(move || {
        if let Err(e) = logic::run_logic_loop(input_rx, ui_tx) {
            eprintln!("Error in Logic Thread: {:?}", e);
        }
    });

    // Start Input Interception Hook directly on the main thread.
    // Windows message pumps need to live on the thread executing hook registrations.
    if let Err(e) = input::run_input_hook(input_tx) {
        eprintln!("Error in Input Interception Hook: {:?}", e);
    }

    // Await thread completion if message pump finishes
    let _ = logic_handle.join();
    let _ = ui_handle.join();
}
