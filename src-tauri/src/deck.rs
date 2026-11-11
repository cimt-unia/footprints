//! Drives a Stream Deck through an OpenDeck plugin, see `docs/stream-deck.md`.
//!
//! The frontend reports which state the run needs the deck in, the backend turns it into what
//! the deck shows, with every text in the language of the study, and serves that on a local
//! WebSocket. The plugin connects, draws whatever it is sent and reports key presses back. A
//! press only counts if the current state offers that key, so a stray press can never advance
//! the run. Accepted presses reach the frontend as the key they stand for, the screens handle
//! them like typing.

use std::sync::LazyLock;

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

/// The state the run needs the deck in, as the frontend reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeckState {
    /// Nothing to press.
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

/// The language of the study, `Settings.lang` in the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    De,
    En,
}

/// The deck's texts, the `deck` section of the frontend's lang files. They stay there so every
/// text of the study is in one place per language.
#[derive(Debug, Deserialize)]
struct Texts {
    enter: String,
    valence: ScaleTexts,
    arousal: ScaleTexts,
}

#[derive(Debug, Deserialize)]
struct ScaleTexts {
    title: String,
    /// The anchors of the values 1 to 7.
    labels: [String; 7],
}

fn parse_texts(lang_file: &str) -> Texts {
    #[derive(Deserialize)]
    struct LangFile {
        deck: Texts,
    }
    serde_json::from_str::<LangFile>(lang_file)
        .expect("the lang file has a valid `deck` section")
        .deck
}

static DE: LazyLock<Texts> =
    LazyLock::new(|| parse_texts(include_str!("../../src/lib/lang/de.json")));
static EN: LazyLock<Texts> =
    LazyLock::new(|| parse_texts(include_str!("../../src/lib/lang/en.json")));

fn texts(lang: Lang) -> &'static Texts {
    match lang {
        Lang::De => &DE,
        Lang::En => &EN,
    }
}

/// What the deck shows, as the plugin receives it: the state with its texts filled in.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum DeckView {
    #[default]
    Default,
    /// Only Enter, labelled `enter`.
    Confirm { enter: String },
    /// `title` names the scale, `labels` are the anchors of the values 1 to 7.
    Rating {
        scale: Scale,
        value: u8,
        title: String,
        labels: [String; 7],
        enter: String,
    },
}

fn view(state: DeckState, lang: Lang) -> DeckView {
    let texts = texts(lang);
    match state {
        DeckState::Default => DeckView::Default,
        DeckState::Confirm => DeckView::Confirm {
            enter: texts.enter.clone(),
        },
        DeckState::Rating { scale, value } => {
            let scale_texts = match scale {
                Scale::Valence => &texts.valence,
                Scale::Arousal => &texts.arousal,
            };
            DeckView::Rating {
                scale,
                value,
                title: scale_texts.title.clone(),
                labels: scale_texts.labels.clone(),
                enter: texts.enter.clone(),
            }
        }
    }
}

/// App to plugin.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Outgoing {
    State { state: DeckView },
}

/// Plugin to app.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Incoming {
    /// `key` is `"1"` to `"7"`, `"enter"` or `"title"`.
    Press { key: String },
}

/// What the deck shows, managed by Tauri. Every connection watches it.
pub struct Deck {
    view: watch::Sender<DeckView>,
}

impl Default for Deck {
    fn default() -> Self {
        Self {
            view: watch::channel(DeckView::default()).0,
        }
    }
}

impl Deck {
    /// Only an actual change reaches the plugin.
    pub fn set(&self, state: DeckState, lang: Lang) {
        let next = view(state, lang);
        self.view.send_if_modified(|current| {
            if *current == next {
                return false;
            }
            *current = next;
            true
        });
    }

    fn current(&self) -> DeckView {
        self.view.borrow().clone()
    }
}

/// The key a press of `key` stands for in `view`, or `None` if it does not offer it.
fn keystroke(view: &DeckView, key: &str) -> Option<&'static str> {
    match (view, key) {
        (DeckView::Confirm { .. } | DeckView::Rating { .. }, "enter") => Some("Enter"),
        (DeckView::Rating { .. }, digit) => match digit {
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
pub fn set_deck_state(state: DeckState, lang: Lang, deck: State<'_, Deck>) {
    deck.set(state, lang);
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
    let mut view = app.state::<Deck>().view.subscribe();
    // A freshly connected plugin knows nothing yet, it gets the current state right away.
    view.mark_changed();
    loop {
        tokio::select! {
            changed = view.changed() => {
                if changed.is_err() {
                    break;
                }
                let state = view.borrow_and_update().clone();
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
    let Some(keystroke) = keystroke(&app.state::<Deck>().current(), &key) else {
        log::info!("Stream Deck press {key:?} ignored, the deck does not offer it right now");
        return;
    };
    log::info!("Stream Deck press {key:?}");
    if let Err(e) = app.emit(KEY_EVENT, keystroke) {
        log::error!("Could not forward a Stream Deck press: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const RATING: DeckState = DeckState::Rating {
        scale: Scale::Arousal,
        value: 4,
    };

    #[test]
    fn both_lang_files_have_the_deck_texts() {
        assert_eq!(texts(Lang::De).valence.title, "Valenz");
        assert_eq!(texts(Lang::De).arousal.labels[0], "Sehr\nniedrig");
        assert_eq!(texts(Lang::En).valence.title, "Valence");
        assert_eq!(texts(Lang::En).arousal.labels[6], "Very\nhigh");
    }

    #[test]
    fn frontend_states_deserialize() {
        let state: DeckState =
            serde_json::from_value(json!({"kind": "rating", "scale": "arousal", "value": 4}))
                .unwrap();
        assert_eq!(state, RATING);
        let lang: Lang = serde_json::from_value(json!("en")).unwrap();
        assert_eq!(lang, Lang::En);
    }

    #[test]
    fn views_serialize_as_documented() {
        let message = |state, lang| {
            serde_json::to_value(Outgoing::State {
                state: view(state, lang),
            })
            .unwrap()
        };
        assert_eq!(
            message(DeckState::Default, Lang::De),
            json!({"type": "state", "state": {"kind": "default"}})
        );
        assert_eq!(
            message(DeckState::Confirm, Lang::De),
            json!({"type": "state", "state": {"kind": "confirm", "enter": "Enter"}})
        );
        assert_eq!(
            message(RATING, Lang::De),
            json!({"type": "state", "state": {
                "kind": "rating", "scale": "arousal", "value": 4, "title": "Erregung",
                "labels": ["Sehr\nniedrig", "Mäßig\nniedrig", "Etwas\nniedrig",
                    "Weder\nniedrig\nnoch\nhoch", "Etwas\nhoch", "Mäßig\nhoch", "Sehr\nhoch"],
                "enter": "Enter",
            }})
        );
    }

    #[test]
    fn presses_parse() {
        let press: Incoming = serde_json::from_str(r#"{"type":"press","key":"3"}"#).unwrap();
        assert_eq!(press, Incoming::Press { key: "3".into() });
    }

    #[test]
    fn only_offered_keys_count() {
        let default = view(DeckState::Default, Lang::De);
        let confirm = view(DeckState::Confirm, Lang::De);
        let rating = view(RATING, Lang::De);
        assert_eq!(keystroke(&default, "enter"), None);
        assert_eq!(keystroke(&default, "3"), None);
        assert_eq!(keystroke(&confirm, "enter"), Some("Enter"));
        assert_eq!(keystroke(&confirm, "3"), None);
        assert_eq!(keystroke(&rating, "enter"), Some("Enter"));
        assert_eq!(keystroke(&rating, "1"), Some("1"));
        assert_eq!(keystroke(&rating, "7"), Some("7"));
        assert_eq!(keystroke(&rating, "8"), None);
        assert_eq!(keystroke(&rating, "title"), None);
    }

    #[test]
    fn repeated_states_are_not_sent_again() {
        let deck = Deck::default();
        let mut watcher = deck.view.subscribe();
        deck.set(DeckState::Default, Lang::De);
        assert!(!watcher.has_changed().unwrap());
        deck.set(DeckState::Confirm, Lang::De);
        assert!(watcher.has_changed().unwrap());
        watcher.mark_unchanged();
        deck.set(DeckState::Confirm, Lang::De);
        assert!(!watcher.has_changed().unwrap());
        // What counts is what the deck shows: both languages label Enter alike...
        deck.set(DeckState::Confirm, Lang::En);
        assert!(!watcher.has_changed().unwrap());
        // ...but not the scales.
        deck.set(RATING, Lang::De);
        watcher.mark_unchanged();
        deck.set(RATING, Lang::En);
        assert!(watcher.has_changed().unwrap());
    }
}
