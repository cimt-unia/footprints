use crate::image_manager::Magnitude;
use crate::lsl::{BlockType, SpeedModifier};
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
        block: rating.block,
        trial_in_block: rating.trial_in_block,
        block_type: rating.block_type,
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
    /// 1 based index of the block this trial belongs to.
    block: usize,
    /// 1 based index of the trial inside its block.
    trial_in_block: usize,
    /// Which kind of block the row belongs to, the same type the LsL marker carries.
    block_type: BlockType,
    // A pause has no phases, no walking and nothing to rate, so it fills none of these.
    baseline_time: Option<DateTime<Local>>,
    stimulus_time: Option<DateTime<Local>>,
    go_time: Option<DateTime<Local>>,
    rating_time: Option<DateTime<Local>>,
    baseline_speed: Option<f64>,
    modification: SpeedModifier,
    effective_speed: Option<f64>,
    // The image fields are absent on trials without a stimulus, those show a fixation cross
    // and are not rated.
    name: Option<String>,
    n_valence: Option<Magnitude>,
    n_arousal: Option<Magnitude>,
    valence: Option<u8>,
    arousal: Option<u8>,
}

// The field order is the CSV column order. Everything a pause does not have is optional and
// comes out as an empty cell.
#[derive(Serialize, Debug)]
struct LogData {
    time: DateTime<Local>,
    block: usize,
    trial_in_block: usize,
    block_type: BlockType,
    baseline_time: Option<DateTime<Local>>,
    stimulus_time: Option<DateTime<Local>>,
    go_time: Option<DateTime<Local>>,
    rating_time: Option<DateTime<Local>>,
    baseline_speed: Option<f64>,
    modification: SpeedModifier,
    modified_speed: Option<f64>,
    picture: Option<String>,
    n_valence: Option<Magnitude>,
    n_arousal: Option<Magnitude>,
    valence: Option<u8>,
    arousal: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pause row carries nothing but its position, every other column has to come out empty.
    /// The frontend simply leaves those keys out, so this pins that a missing key is `None`
    /// rather than a deserialize error.
    #[test]
    fn pause_row_needs_only_its_position() {
        let json = r#"{
            "block": 2,
            "trial_in_block": 1,
            "block_type": "pause",
            "modification": "none"
        }"#;
        let rating: Rating = serde_json::from_str(json).unwrap();
        assert_eq!(rating.block, 2);
        assert!(rating.baseline_time.is_none());
        assert!(rating.rating_time.is_none());
        assert!(rating.baseline_speed.is_none());
        assert!(rating.effective_speed.is_none());
        assert!(rating.name.is_none());
    }

    /// A static trial has no go phase and no walking, so those columns stay empty. Pins that the
    /// row is accepted at all: a rejected `add_rating` drops the trial from the CSV.
    #[test]
    fn static_stimulus_row_has_no_walking() {
        let json = r#"{
            "block": 1,
            "trial_in_block": 2,
            "block_type": "static_stimulus",
            "baseline_time": "2026-08-21T10:00:00+02:00",
            "stimulus_time": "2026-08-21T10:00:02+02:00",
            "rating_time": "2026-08-21T10:00:09+02:00",
            "baseline_speed": 4.0,
            "modification": "none",
            "valence": 3,
            "arousal": 5
        }"#;
        let rating: Rating = serde_json::from_str(json).unwrap();
        assert_eq!(rating.modification, SpeedModifier::None);
        assert!(rating.go_time.is_none());
        assert!(rating.effective_speed.is_none());
    }

    /// The speed reads the same in the log as it does in the marker.
    #[test]
    fn trial_row_speed_matches_the_marker() {
        let json = r#"{
            "block": 1,
            "trial_in_block": 3,
            "block_type": "neutral",
            "baseline_time": "2026-08-21T10:00:00+02:00",
            "stimulus_time": "2026-08-21T10:00:02+02:00",
            "go_time": "2026-08-21T10:00:05+02:00",
            "rating_time": "2026-08-21T10:00:09+02:00",
            "baseline_speed": 4.0,
            "modification": "very_slow",
            "effective_speed": 3.3
        }"#;
        let rating: Rating = serde_json::from_str(json).unwrap();
        assert_eq!(rating.modification, SpeedModifier::VerySlow);
        assert!(rating.valence.is_none());
    }
}
