//! The messages between the footprints app and its Stream Deck plugin, see footprints'
//! `docs/stream-deck.md`.
//!
//! The app (`src-tauri`) serves a WebSocket on `127.0.0.1:`[`PORT`], the plugin
//! (`stream-deck-plugin`) connects to it. Each text frame is one JSON message: [`AppMessage`]
//! from the app to the plugin, [`PluginMessage`] back. Both sides build against this crate, so
//! they cannot disagree on the format; its tests pin the format to what the doc shows.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The port the app serves the plugin on, on `127.0.0.1` only.
pub const PORT: u16 = 47823;

/// App to plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppMessage {
    /// What the deck shows. Sent right after the plugin connects and whenever it changes.
    State { state: DeckView },
}

/// Plugin to app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginMessage {
    /// `key` was pressed. The app ignores a press the current view does not offer.
    Press { key: Key },
}

/// What the deck shows, with every text in the language of the study.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeckView {
    /// Nothing to press: every footprints key is blank.
    #[default]
    Default,
    /// Only Enter, labelled `enter`.
    Confirm { enter: String },
    /// The values of one scale with `value` (1 to 7) selected, and Enter to confirm it. `title`
    /// names the scale, `labels` are the anchors of the values 1 to 7, `enter` labels Enter.
    Rating {
        scale: Scale,
        value: u8,
        title: String,
        labels: [String; 7],
        enter: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    Valence,
    Arousal,
}

/// A footprints key: one of the seven values, Enter, or the key naming the scale. On the wire
/// it is `"1"` to `"7"`, `"enter"` or `"title"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "&'static str", try_from = "String")]
pub enum Key {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Enter,
    Title,
}

impl Key {
    /// The seven values, 1 to 7.
    pub const VALUES: [Key; 7] = [
        Key::One,
        Key::Two,
        Key::Three,
        Key::Four,
        Key::Five,
        Key::Six,
        Key::Seven,
    ];

    /// The value `n` (1 to 7), if it is one.
    pub fn value(n: u8) -> Option<Key> {
        Key::VALUES.get(usize::from(n).checked_sub(1)?).copied()
    }

    /// The value this key stands for, 1 to 7, or `None` for Enter and the title.
    pub fn number(self) -> Option<u8> {
        Key::VALUES
            .iter()
            .position(|&key| key == self)
            .map(|i| i as u8 + 1)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Key::One => "1",
            Key::Two => "2",
            Key::Three => "3",
            Key::Four => "4",
            Key::Five => "5",
            Key::Six => "6",
            Key::Seven => "7",
            Key::Enter => "enter",
            Key::Title => "title",
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownKey(pub String);

impl fmt::Display for UnknownKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown footprints key {:?}, expected 1 to 7, enter or title",
            self.0
        )
    }
}

impl std::error::Error for UnknownKey {}

/// Exactly the wire form; a lenient parse of what a user typed is the plugin's business.
impl FromStr for Key {
    type Err = UnknownKey;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "enter" => Ok(Key::Enter),
            "title" => Ok(Key::Title),
            _ => s
                .parse::<u8>()
                .ok()
                .filter(|_| s.len() == 1)
                .and_then(Key::value)
                .ok_or_else(|| UnknownKey(s.to_owned())),
        }
    }
}

impl From<Key> for &'static str {
    fn from(key: Key) -> Self {
        key.as_str()
    }
}

impl TryFrom<String> for Key {
    type Error = UnknownKey;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    /// `message` serializes to `wire` and parses back from it.
    fn round_trips<T>(message: T, wire: Value)
    where
        T: Serialize + for<'de> Deserialize<'de> + PartialEq + fmt::Debug,
    {
        assert_eq!(serde_json::to_value(&message).unwrap(), wire);
        assert_eq!(serde_json::from_value::<T>(wire).unwrap(), message);
    }

    // The examples of docs/stream-deck.md.

    #[test]
    fn states_are_as_documented() {
        round_trips(
            AppMessage::State {
                state: DeckView::Default,
            },
            json!({"type": "state", "state": {"kind": "default"}}),
        );
        round_trips(
            AppMessage::State {
                state: DeckView::Confirm {
                    enter: "Enter".into(),
                },
            },
            json!({"type": "state", "state": {"kind": "confirm", "enter": "Enter"}}),
        );
        let labels = [
            "Sehr\nnegativ",
            "Mäßig\nnegativ",
            "Etwas\nnegativ",
            "Neutral",
            "Etwas\npositiv",
            "Mäßig\npositiv",
            "Sehr\npositiv",
        ];
        round_trips(
            AppMessage::State {
                state: DeckView::Rating {
                    scale: Scale::Valence,
                    value: 4,
                    title: "Valenz".into(),
                    labels: labels.map(String::from),
                    enter: "Enter".into(),
                },
            },
            json!({"type": "state", "state": {
                "kind": "rating", "scale": "valence", "value": 4, "title": "Valenz",
                "labels": labels, "enter": "Enter",
            }}),
        );
        assert_eq!(serde_json::to_value(Scale::Arousal).unwrap(), "arousal");
    }

    #[test]
    fn presses_are_as_documented() {
        round_trips(
            PluginMessage::Press { key: Key::Three },
            json!({"type": "press", "key": "3"}),
        );
        round_trips(
            PluginMessage::Press { key: Key::Enter },
            json!({"type": "press", "key": "enter"}),
        );
        round_trips(
            PluginMessage::Press { key: Key::Title },
            json!({"type": "press", "key": "title"}),
        );
    }

    #[test]
    fn unknown_keys_do_not_parse() {
        for wire in ["0", "8", "03", "+3", "Enter", " enter", ""] {
            assert!(
                serde_json::from_value::<PluginMessage>(json!({"type": "press", "key": wire}))
                    .is_err(),
                "{wire:?} parsed"
            );
        }
        assert!(serde_json::from_value::<Key>(json!(3)).is_err());
    }

    #[test]
    fn keys_know_their_values() {
        for n in 1..=7 {
            let key = Key::value(n).unwrap();
            assert_eq!(key.number(), Some(n));
            assert_eq!(key.as_str().parse::<Key>(), Ok(key));
        }
        assert_eq!(Key::value(0), None);
        assert_eq!(Key::value(8), None);
        assert_eq!(Key::Enter.number(), None);
        assert_eq!(Key::Title.number(), None);
        assert_eq!("enter".parse::<Key>(), Ok(Key::Enter));
        assert_eq!("title".parse::<Key>(), Ok(Key::Title));
    }
}
