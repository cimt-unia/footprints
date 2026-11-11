//! Drives a Stream Deck through an OpenDeck plugin, see `docs/stream-deck.md`.
//!
//! The app holds the state the deck should be in and serves it on a local WebSocket. The plugin
//! connects, draws whatever state it is sent and reports key presses back. A press only counts
//! if the current state offers that key, so a stray press can never advance the run. Accepted
//! presses reach the frontend as the key they stand for, the screens handle them like typing.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

/// The port the plugin connects to, on `127.0.0.1` only.
pub const PORT: u16 = 47823;

/// The event an accepted press is emitted as. Its payload is the key name the press stands
/// for (`"1"` to `"7"` or `"Enter"`), as in `KeyboardEvent.key`.
pub const KEY_EVENT: &str = "deck-key";

/// What the deck shows and which of its keys do anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeckState {
    /// Nothing to press.
    #[default]
    Default,
    /// Only Enter.
    Confirm,
    /// The 1 to 7 keys of one scale with `value` selected, and Enter to confirm it.
    Rating { scale: Scale, value: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    Valence,
    Arousal,
}

/// App to plugin.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Outgoing {
    State { state: DeckState },
}

/// Plugin to app.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Incoming {
    /// `key` is `"1"` to `"7"` or `"enter"`.
    Press { key: String },
}

/// The deck state, managed by Tauri. Every connection watches it.
pub struct Deck {
    state: watch::Sender<DeckState>,
}

impl Default for Deck {
    fn default() -> Self {
        Self {
            state: watch::channel(DeckState::default()).0,
        }
    }
}

impl Deck {
    /// Only an actual change reaches the plugin.
    pub fn set(&self, next: DeckState) {
        self.state.send_if_modified(|current| {
            if *current == next {
                return false;
            }
            *current = next;
            true
        });
    }

    fn current(&self) -> DeckState {
        *self.state.borrow()
    }
}

/// The key a press of `key` stands for in `state`, or `None` if the state does not offer it.
fn keystroke(state: DeckState, key: &str) -> Option<&'static str> {
    match (state, key) {
        (DeckState::Confirm | DeckState::Rating { .. }, "enter") => Some("Enter"),
        (DeckState::Rating { .. }, digit) => match digit {
            "1" => Some("1"),
            "2" => Some("2"),
            "3" => Some("3"),
            "4" => Some("4"),
            "5" => Some("5"),
            "6" => Some("6"),
            "7" => Some("7"),
            _ => None,
        },
        _ => None,
    }
}

#[tauri::command]
pub fn set_deck_state(state: DeckState, deck: State<'_, Deck>) {
    deck.set(state);
}

/// Starts the server. Without the port the app runs on, only the deck stays dark.
pub fn serve(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let listener = match TcpListener::bind(("127.0.0.1", PORT)).await {
            Ok(listener) => listener,
            Err(e) => {
                log::error!("The Stream Deck server cannot listen on port {PORT}: {e}");
                return;
            }
        };
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    tauri::async_runtime::spawn(connection(app.clone(), stream));
                }
                Err(e) => log::warn!("A Stream Deck connection failed: {e}"),
            }
        }
    });
}

async fn connection(app: AppHandle, stream: TcpStream) {
    let ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            log::warn!("A Stream Deck handshake failed: {e}");
            return;
        }
    };
    log::info!("Stream Deck plugin connected");
    let (mut outgoing, mut incoming) = ws.split();
    let mut state = app.state::<Deck>().state.subscribe();
    // A freshly connected plugin knows nothing yet, it gets the current state right away.
    state.mark_changed();
    loop {
        tokio::select! {
            changed = state.changed() => {
                if changed.is_err() {
                    break;
                }
                let state = *state.borrow_and_update();
                let message = serde_json::to_string(&Outgoing::State { state })
                    .expect("the deck state serializes");
                if outgoing.send(Message::text(message)).await.is_err() {
                    break;
                }
            }
            message = incoming.next() => match message {
                Some(Ok(Message::Text(text))) => on_message(&app, text.as_str()),
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                Some(Ok(_)) => {}
            },
        }
    }
    log::info!("Stream Deck plugin disconnected");
}

fn on_message(app: &AppHandle, text: &str) {
    let Incoming::Press { key } = match serde_json::from_str(text) {
        Ok(message) => message,
        Err(e) => {
            log::warn!("Unknown Stream Deck message {text:?}: {e}");
            return;
        }
    };
    let Some(keystroke) = keystroke(app.state::<Deck>().current(), &key) else {
        return;
    };
    if let Err(e) = app.emit(KEY_EVENT, keystroke) {
        log::error!("Could not forward a Stream Deck press: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const RATING: DeckState = DeckState::Rating {
        scale: Scale::Valence,
        value: 4,
    };

    #[test]
    fn states_serialize_as_documented() {
        let message = |state| serde_json::to_value(Outgoing::State { state }).unwrap();
        assert_eq!(
            message(DeckState::Default),
            json!({"type": "state", "state": {"kind": "default"}})
        );
        assert_eq!(
            message(DeckState::Confirm),
            json!({"type": "state", "state": {"kind": "confirm"}})
        );
        assert_eq!(
            message(RATING),
            json!({"type": "state", "state": {"kind": "rating", "scale": "valence", "value": 4}})
        );
    }

    #[test]
    fn frontend_states_deserialize() {
        let state: DeckState =
            serde_json::from_value(json!({"kind": "rating", "scale": "arousal", "value": 7}))
                .unwrap();
        assert_eq!(
            state,
            DeckState::Rating {
                scale: Scale::Arousal,
                value: 7
            }
        );
    }

    #[test]
    fn presses_parse() {
        let press: Incoming = serde_json::from_str(r#"{"type":"press","key":"3"}"#).unwrap();
        assert_eq!(press, Incoming::Press { key: "3".into() });
    }

    #[test]
    fn only_offered_keys_count() {
        assert_eq!(keystroke(DeckState::Default, "enter"), None);
        assert_eq!(keystroke(DeckState::Default, "3"), None);
        assert_eq!(keystroke(DeckState::Confirm, "enter"), Some("Enter"));
        assert_eq!(keystroke(DeckState::Confirm, "3"), None);
        assert_eq!(keystroke(RATING, "enter"), Some("Enter"));
        assert_eq!(keystroke(RATING, "1"), Some("1"));
        assert_eq!(keystroke(RATING, "7"), Some("7"));
        assert_eq!(keystroke(RATING, "8"), None);
        assert_eq!(keystroke(RATING, "0"), None);
    }

    #[test]
    fn repeated_states_are_not_sent_again() {
        let deck = Deck::default();
        let mut watcher = deck.state.subscribe();
        deck.set(DeckState::Default);
        assert!(!watcher.has_changed().unwrap());
        deck.set(DeckState::Confirm);
        assert!(watcher.has_changed().unwrap());
        watcher.mark_unchanged();
        deck.set(DeckState::Confirm);
        assert!(!watcher.has_changed().unwrap());
    }
}
