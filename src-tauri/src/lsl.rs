use crate::image_manager::Magnitude;
use log::{error, info};
use lsl::{Pushable, StreamOutlet};
use serde::{Deserialize, Serialize};
use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread::spawn,
};

pub struct LsLManager {
    sender: Sender<String>,
}

pub struct LsL {
    recv: Receiver<String>,
    event_outlet: StreamOutlet,
}

// One App event, published to LsL as a JSON object on a string channel.
//
// The enum is tagged on the block that emitted the marker because that is what decides which
// fields a marker has: only a stimulus trial shows an image and only a stimulus trial is rated,
// a pause has neither trial nor phase, and only a calibration has a step and a result speed.
// The phase inside the block is a second tag (`state`) flattened into the variant, so a rating
// can only ever ride along with a rating phase.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LsLMarker {
    // The start of a session. It belongs to no block and no trial.
    Session,
    // A break between blocks, it has no phases and nothing to record but which block it follows.
    Pause {
        block: u8,
    },
    // A step of the speed calibration. It has no block, the step is its own counter.
    Calibration {
        step: u8,
        #[serde(flatten)]
        event: CalibrationEvent,
    },
    Neutral(NeutralTrial),
    Stimulus(StimulusTrial),
    // A practice run opened from the instructions. It runs an ordinary stimulus plan, the tag is
    // what keeps a recording from confusing it with real data.
    Test(StimulusTrial),
}

// A trial that shows an emotional image, the only kind that is rated.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StimulusTrial {
    // 1 based index of the block inside the plan.
    block: u8,
    // 1 based index of the trial inside its block.
    trial: u8,
    // Walking condition of the trial.
    #[serde(default)]
    speed: SpeedModifier,
    // Absent when the image of the trial failed to load.
    image_data: Option<ImageData>,
    #[serde(flatten)]
    event: StimulusEvent,
}

// A trial that shows a fixation cross in the image's place. It has no image and is never rated,
// the subject only confirms.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NeutralTrial {
    block: u8,
    trial: u8,
    #[serde(default)]
    speed: SpeedModifier,
    #[serde(flatten)]
    event: NeutralEvent,
}

// The phases of a stimulus trial. The two rating phases carry the rating the subject gave.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum StimulusEvent {
    Baseline,
    Stimulus,
    Go,
    RatingPrompt,
    RatingValence { rating: u8 },
    RatingArousal { rating: u8 },
}

// The phases of a neutral trial, the same moments as a stimulus trial minus the ratings.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum NeutralEvent {
    Baseline,
    Stimulus,
    Go,
    RatingPrompt,
}

// What happened to a calibration step. `Result` closes a calibration and is the only one that
// carries data.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CalibrationEvent {
    Start,
    Stop,
    Discarded,
    Confirmed,
    Result { speed_kmh: f64 },
}

// The image of a stimulus trial, as the quadrant it was drawn from and its index inside that
// quadrant. The frontend holds the identifier in the packed form `image_manager` hands out, so
// the marker takes that number on the wire and unpacks it for the recording.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(from = "u16")]
pub struct ImageData {
    valence: Magnitude,
    arousal: Magnitude,
    index: u16,
}

impl From<u16> for ImageData {
    fn from(id: u16) -> Self {
        // The quadrants are ordered low_low, low_high, high_low, high_high, so the upper of the
        // two quadrant bits is the valence and the lower one the arousal, see `get_rand_image`.
        let magnitude = |high: bool| {
            if high {
                Magnitude::High
            } else {
                Magnitude::Low
            }
        };
        Self {
            valence: magnitude(id & (1 << 15) != 0),
            arousal: magnitude(id & (1 << 14) != 0),
            index: id & ((1 << 14) - 1),
        }
    }
}

// The walking condition of a trial. `None` covers the markers that involve no walking at all: a
// pause, a session start and a calibration run.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum SpeedModifier {
    #[default]
    None = 0,
    VerySlow = 1,
    Slow = 2,
    Normal = 3,
    Fast = 4,
    VeryFast = 5,
}

// All currently supported Block types. Calibration, Test and Session are not technically blocks
// but are included so we can tell those runs apart from real data. The CSV log shares this type
// with the marker, its variant names are the column values there.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(u8)]
#[serde(rename_all = "lowercase")]
pub enum BlockType {
    Calibration = 0,
    Stimulus = 1,
    Neutral = 2,
    Pause = 3,
    Test = 4,
    Session = 5,
}

impl LsL {
    fn new(rx: Receiver<String>) -> Result<Self, lsl::Error> {
        let info = lsl::StreamInfo::new(
            "App Events",
            "Markers",
            1,
            lsl::IRREGULAR_RATE,
            lsl::ChannelFormat::String,
            "",
        )?;

        Ok(Self {
            recv: rx,
            event_outlet: StreamOutlet::new(&info, 1, 360)?,
        })
    }

    fn start(&self) {
        loop {
            let e = self.recv.recv().unwrap();
            info!("Received Event: {e}");

            if let Err(e) = self.event_outlet.push_sample(&[e]) {
                error!("Error sending lsl packet: {e:?}");
            }
        }
    }
}

impl LsLManager {
    pub fn new() -> Self {
        let (tx, rx) = channel::<String>();
        spawn(move || {
            let lsl = LsL::new(rx).unwrap();
            lsl.start();
        });
        Self { sender: tx }
    }

    pub fn publish_event(&self, event: LsLMarker) -> anyhow::Result<()> {
        info!("Received Marker: {event:?}");
        // Serializing here and not in the worker so a broken marker is reported to the caller
        // instead of disappearing into the thread.
        let marker = serde_json::to_string(&event)?;
        self.sender.send(marker)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    /// Checks one marker end to end: `sent` is the payload as the frontend hands it to
    /// `publish_lsl`, `published` the JSON that goes out on the stream. Deserializing is what
    /// pins the flattened `state` enums, they sit inside an internally tagged enum which is the
    /// one part of this representation that could silently stop parsing.
    fn publishes(sent: Value, published: Value) {
        let marker: LsLMarker = serde_json::from_value(sent).unwrap();
        assert_eq!(serde_json::to_value(&marker).unwrap(), published);
    }

    /// A session start belongs to no block and no trial, the tag is the whole marker.
    #[test]
    fn session_marker() {
        publishes(
            json!({ "type": "session" }),
            json!({ "type": "session" }),
        );
    }

    /// A pause has no phase and no walking, only the block it follows.
    #[test]
    fn pause_marker() {
        publishes(
            json!({ "type": "pause", "block": 3 }),
            json!({ "type": "pause", "block": 3 }),
        );
    }

    /// The four moments of a calibration step differ in nothing but their state.
    #[test]
    fn calibration_step_markers() {
        for state in ["start", "stop", "discarded", "confirmed"] {
            let marker = json!({ "type": "calibration", "step": 2, "state": state });
            publishes(marker.clone(), marker);
        }
    }

    /// The marker closing a calibration carries the calibrated speed as plain km/h, no longer
    /// as the fixed point number the packed marker had to squeeze into an integer slot.
    #[test]
    fn calibration_result_carries_the_speed() {
        let marker = json!({
            "type": "calibration",
            "step": 3,
            "state": "result",
            "speed_kmh": 4.23,
        });
        publishes(marker.clone(), marker);
    }

    /// A neutral trial shows a fixation cross, so it has no image and is never rated.
    #[test]
    fn neutral_trial_marker() {
        let marker = json!({
            "type": "neutral",
            "block": 2,
            "trial": 5,
            "speed": "slow",
            "state": "go",
        });
        publishes(marker.clone(), marker);
    }

    /// A rating rides along with the phase it belongs to, next to the image it rates. The
    /// frontend sends the identifier packed, the recording gets the quadrant spelled out.
    #[test]
    fn stimulus_rating_marker() {
        publishes(
            json!({
                "type": "stimulus",
                "block": 1,
                "trial": 3,
                "speed": "very_fast",
                "image_id": 1 << 14 | 5,
                "state": "rating_valence",
                "rating": 7,
            }),
            json!({
                "type": "stimulus",
                "block": 1,
                "trial": 3,
                "speed": "very_fast",
                "image_id": { "valence": "Low", "arousal": "High", "index": 5 },
                "state": "rating_valence",
                "rating": 7,
            }),
        );
    }

    /// A trial whose image failed to load still stamps its marker, without an image. The speed
    /// defaults, the frontend leaves it out wherever there is no walking.
    #[test]
    fn stimulus_without_an_image() {
        publishes(
            json!({
                "type": "stimulus",
                "block": 1,
                "trial": 1,
                "image_id": Value::Null,
                "state": "baseline",
            }),
            json!({
                "type": "stimulus",
                "block": 1,
                "trial": 1,
                "speed": "none",
                "image_id": Value::Null,
                "state": "baseline",
            }),
        );
    }

    /// A practice run carries the same payload as a stimulus trial, only the tag tells the two
    /// apart, which is what keeps it out of the real data.
    #[test]
    fn test_run_marker() {
        publishes(
            json!({
                "type": "test",
                "block": 1,
                "trial": 2,
                "speed": "none",
                "image_id": 3 << 14 | 12,
                "state": "stimulus",
            }),
            json!({
                "type": "test",
                "block": 1,
                "trial": 2,
                "speed": "none",
                "image_id": { "valence": "High", "arousal": "High", "index": 12 },
                "state": "stimulus",
            }),
        );
    }

    /// A rating may only appear on a rating phase, and a neutral trial has none at all.
    #[test]
    fn a_neutral_trial_cannot_be_rated() {
        let rated = json!({
            "type": "neutral",
            "block": 2,
            "trial": 5,
            "speed": "slow",
            "state": "rating_valence",
            "rating": 7,
        });
        assert!(serde_json::from_value::<LsLMarker>(rated).is_err());
    }

    /// The identifier packs the quadrant into its upper 2 bits, valence above arousal, matching
    /// the quadrant order of `get_rand_image`.
    #[test]
    fn image_id_unpacks_the_quadrant() {
        for (id, valence, arousal) in [
            (0u16, "Low", "Low"),
            (1 << 14, "Low", "High"),
            (2 << 14, "High", "Low"),
            (3 << 14, "High", "High"),
        ] {
            assert_eq!(
                serde_json::to_value(ImageData::from(id | 5)).unwrap(),
                json!({ "valence": valence, "arousal": arousal, "index": 5 }),
            );
        }
    }

    /// The frontend sends the speed as a snake_case string and the CSV log shares the type, a
    /// rename here would change both.
    #[test]
    fn speed_wire_names() {
        for (speed, name) in [
            (SpeedModifier::None, "none"),
            (SpeedModifier::VerySlow, "very_slow"),
            (SpeedModifier::Slow, "slow"),
            (SpeedModifier::Normal, "normal"),
            (SpeedModifier::Fast, "fast"),
            (SpeedModifier::VeryFast, "very_fast"),
        ] {
            let json = format!("\"{name}\"");
            assert_eq!(serde_json::to_string(&speed).unwrap(), json);
            assert_eq!(serde_json::from_str::<SpeedModifier>(&json).unwrap(), speed);
        }
    }

    /// The block type names are both the marker tags and the CSV column values, a rename here
    /// would silently break every recording.
    #[test]
    fn block_type_wire_names() {
        for (block_type, name) in [
            (BlockType::Calibration, "calibration"),
            (BlockType::Stimulus, "stimulus"),
            (BlockType::Neutral, "neutral"),
            (BlockType::Pause, "pause"),
            (BlockType::Test, "test"),
            (BlockType::Session, "session"),
        ] {
            let json = format!("\"{name}\"");
            assert_eq!(serde_json::to_string(&block_type).unwrap(), json);
            assert_eq!(
                serde_json::from_str::<BlockType>(&json).unwrap() as u8,
                block_type as u8
            );
        }
    }
}
