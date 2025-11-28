mod commands;
mod image_manager;
mod logger;
mod lsl;
mod rand;

use anyhow::{Context, Result};
use commands::{get_image, open_calibration, play_sound, publish_lsl};
use image_manager::ImageManager;
use logger::{add_rating, init_logger, save_experiment, Logger};
use lsl::LsLManager;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<()> {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_log::Builder::new().build());
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = app
                .get_webview_window("main")
                .expect("no main window")
                .set_focus();
        }));
    }
    builder
        .invoke_handler(tauri::generate_handler![
            get_image,
            open_calibration,
            publish_lsl,
            init_logger,
            add_rating,
            save_experiment,
            play_sound
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .setup(|app| {
            if let Err(e) = setup_managed_state(app.handle()) {
                println!("{:?}", e);
                app.handle().exit(1);
            }
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let open_c = MenuItem::with_id(app, "calibration", "Calibration", true, None::<&str>)?;
            let open_i = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_i, &open_c, &quit_i])?;
            TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "open" => {
                        let main_window = app.get_webview_window("main").unwrap();
                        main_window.show().unwrap();
                        main_window.unminimize().unwrap();
                        main_window.set_focus().unwrap();
                    }
                    "calibration" => {
                        let calibration_window = app.get_webview_window("calibration").unwrap();
                        calibration_window.show().unwrap();
                        calibration_window.unminimize().unwrap();
                        calibration_window.set_focus().unwrap();
                    }
                    _ => panic!("Handler for event: {:?} not implemented", event.id()),
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        let app = tray.app_handle();
                        let main_window = app.get_webview_window("main").unwrap();
                        main_window.show().unwrap();
                        main_window.set_focus().unwrap();
                    }
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .with_context(|| "error while running tauri application")?;
    Ok(())
}

fn setup_managed_state(app: &AppHandle) -> Result<()> {
    let image_manager = ImageManager::init(app)?;
    let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
    app.manage(Mutex::new(image_manager));
    app.manage(LsLManager::new());
    let sink = rodio::Sink::connect_new(stream_handle.mixer());
    std::mem::forget(stream_handle);
    app.manage(sink);
    app.manage(LsLManager::new());
    app.manage(Mutex::new(Logger::default()));
    Ok(())
}
