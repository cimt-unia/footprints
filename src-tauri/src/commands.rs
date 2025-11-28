use tauri::{path::BaseDirectory, AppHandle, Manager, State};

use crate::{
    image_manager::{Image, ImageManager},
    lsl::{LsLEvent, LsLManager},
};
use std::sync::Mutex;
use rodio::Sink;

#[tauri::command]
pub fn get_image(init: bool, state: State<'_, Mutex<ImageManager>>) -> Image {
    state.lock().unwrap().get_rand_image(init).clone()
}

#[tauri::command]
pub fn open_calibration(app: AppHandle) {
    let calibration_window = app.get_webview_window("calibration").unwrap();
    calibration_window.show().unwrap();
    calibration_window.unminimize().unwrap();
    calibration_window.set_focus().unwrap();
}

#[tauri::command]
pub fn publish_lsl(
    event: LsLEvent,
    state: State<'_, LsLManager>,
) {
    state.publish_event(event).expect("Lsl worker crashed");
}

#[tauri::command]
pub fn play_sound(app: AppHandle, sink: State<'_, Sink>) {
    let mut path = app
        .path()
        .resolve("resources/music/", BaseDirectory::Resource)
        .expect("music folder to be present");
    path.push("start-13691.mp3");
    let file = std::fs::File::open(path).unwrap();
    sink.append(rodio::Decoder::try_from(file).unwrap());
}
