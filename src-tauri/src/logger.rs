use crate::image_manager::Magnitude;
use chrono::{DateTime, Local};
use log::info;
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, File},
    sync::Mutex,
};
use tauri::{path::BaseDirectory, AppHandle, Manager, State};

#[derive(Debug, Default)]
pub struct Logger {
    subject: Option<String>,
    logs: Vec<LogData>,
}

#[tauri::command]
pub fn init_logger(name: String, state: State<'_, Mutex<Logger>>) {
    state.lock().unwrap().subject = Some(name);
}

#[tauri::command]
pub fn add_rating(rating: Rating, state: State<'_, Mutex<Logger>>) {
    let mut logger = state.lock().unwrap();
    let data = LogData {
        time: Local::now(),
        baseline_speed: rating.baseline_speed,
        modification: rating.modification,
        modified_speed: rating.effective_speed,
        picture: rating.name,
        valence: rating.valence,
        arousal: rating.arousal,
        baseline_time: rating.baseline_time,
        stimulus_time: rating.stimulus_time,
        go_time: rating.go_time,
        rating_time: rating.rating_time,
        n_valence: rating.n_valence,
        n_arousal: rating.n_arousal,
    };
    info!("logging");
    logger.logs.push(data);
}

#[tauri::command]
pub fn save_experiment(study: String, state: State<'_, Mutex<Logger>>, app: AppHandle) {
    let mut p = app
        .path()
        .resolve(format!("footprints/{study}"), BaseDirectory::Home)
        .unwrap();

    create_dir_all(&p).unwrap();
    let mut logger = state.lock().unwrap();
    if logger.logs.is_empty() {
        info!("No loggs");
        return;
    }
    p.push(format!(
        "{}_{}.csv",
        logger.subject.as_ref().unwrap(),
        Local::now().timestamp()
    ));
    let f = File::create(p).unwrap();

    let mut wtr = csv::Writer::from_writer(f);

    // When writing records with Serde using structs, the header row is written
    // automatically.
    for log in logger.logs.drain(..) {
        wtr.serialize(log).unwrap();
    }
    wtr.flush().unwrap();
}

#[derive(Deserialize)]
pub struct Rating {
    baseline_time: DateTime<Local>,
    stimulus_time: DateTime<Local>,
    go_time: DateTime<Local>,
    rating_time: DateTime<Local>,
    baseline_speed: f64,
    modification: SpeedModification,
    effective_speed: f64,
    name: String,
    n_valence: Magnitude,
    n_arousal: Magnitude,
    valence: Option<u8>,
    arousal: Option<u8>,
}

#[derive(Serialize, Debug)]
struct LogData {
    time: DateTime<Local>,
    baseline_time: DateTime<Local>,
    stimulus_time: DateTime<Local>,
    go_time: DateTime<Local>,
    rating_time: DateTime<Local>,
    baseline_speed: f64,
    modification: SpeedModification,
    modified_speed: f64,
    picture: String,
    n_valence: Magnitude,
    n_arousal: Magnitude,
    valence: Option<u8>,
    arousal: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
enum SpeedModification {
    #[serde(rename = "Very slow")]
    VerySlow,
    Slow,
    Normal,
    Fast,
    #[serde(rename = "Very fast")]
    VeryFast,
}
