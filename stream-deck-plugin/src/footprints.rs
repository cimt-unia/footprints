//! Lets the footprints app (0.3.0 or later) drive the keys.
//!
//! The app serves a WebSocket on `127.0.0.1`. This module stays connected to it, retrying while
//! the app is not running. Keys whose `footprints` setting names a footprints key (`1` to `7`,
//! `enter` or `title`) show what the app sends, and their presses go to the app. Every text comes
//! from the app, in the language of the study, so one page serves both scales and every language.
//! Without the app they are black and do nothing.
//!
//! The app says which protocol it speaks right after connecting. If it is not this plugin's, the
//! keys stay off and the scale's name says which of the two to update.
//!
//! The protocol is documented in footprints' `docs/stream-deck.md`, its messages are the
//! `deck-protocol` crate the app builds against too.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::Ordering;
use std::time::Duration;

use deck_protocol::{
    AppMessage, DeckView, Key, PORT, PROTOCOL, PluginMessage, UNANNOUNCED_PROTOCOL,
};
use futures_util::{SinkExt, StreamExt};
use openaction::{Instance, InstanceId, get_instance};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const RETRY: Duration = Duration::from_secs(2);

/// The footprints key a setting names, if it names one. Unlike the wire, a setting is typed by
/// hand, so case and surrounding blanks do not matter.
pub fn parse_key(setting: Option<&str>) -> Option<Key> {
    setting?.trim().to_lowercase().parse().ok()
}

/// The key colours, as 1x1 images OpenDeck stretches over the key. A key that does nothing in
/// the current state is black; otherwise the values are white, green while selected, Enter is
/// orange and the scale's name stays black.
const BLACK: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGNgYGD4DwABBAEAX+XDSwAAAABJRU5ErkJggg==";
const WHITE: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR4nGP4DwQACfsD/fteaysAAAAASUVORK5CYII=";
const GREEN: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGPQOFz1HwAE4QJlJviqGAAAAABJRU5ErkJggg==";
const ORANGE: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4X8vwHwAGdwJ8AvEzFwAAAABJRU5ErkJggg==";

/// What one key shows.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Look {
    title: String,
    /// Selected, drawn in the Value action's second state.
    lit: bool,
    /// Does something in the current state. A key that does not is black.
    active: bool,
}

impl Look {
    fn active(title: &str, lit: bool) -> Self {
        Self {
            title: title.to_owned(),
            lit,
            active: true,
        }
    }

    fn off() -> Self {
        Self {
            title: String::new(),
            lit: false,
            active: false,
        }
    }
}

/// What the footprints keys show.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(clippy::large_enum_variant)] // There is only ever the one in `LINK`.
enum Shown {
    /// Not connected, or nothing received yet: every key is off.
    #[default]
    Off,
    /// What the app sent.
    View(DeckView),
    /// The app speaks protocol `app`, not this plugin's. The scale's name says which of the two
    /// to update, every other key is off and presses are not sent.
    Mismatch { app: u32 },
}

/// What a message from the app makes the keys show, or `None` if it changes nothing.
/// `app_protocol` is the protocol the app speaks, as far as it said so.
fn receive(app_protocol: &mut u32, message: AppMessage) -> Option<Shown> {
    match message {
        AppMessage::Hello { protocol } => {
            *app_protocol = protocol;
            (protocol != PROTOCOL).then_some(Shown::Mismatch { app: protocol })
        }
        AppMessage::State { state } => Some(if *app_protocol == PROTOCOL {
            Shown::View(state)
        } else {
            Shown::Mismatch { app: *app_protocol }
        }),
    }
}

/// What `key` shows while the keys show `shown`.
fn look(shown: &Shown, key: Key) -> Look {
    match (shown, key) {
        (Shown::Off, _) => Look::off(),
        (Shown::View(view), key) => view_look(view, key),
        // An app with a newer protocol than this plugin's needs a newer plugin, an older one
        // needs updating itself.
        (Shown::Mismatch { app }, Key::Title) if *app > PROTOCOL => {
            Look::active("Update\nplugin", false)
        }
        (Shown::Mismatch { .. }, Key::Title) => Look::active("Update\nfootprints", false),
        (Shown::Mismatch { .. }, _) => Look::off(),
    }
}

/// What `key` shows in `view`.
fn view_look(view: &DeckView, key: Key) -> Look {
    match (view, key) {
        (DeckView::Confirm { enter } | DeckView::Rating { enter, .. }, Key::Enter) => {
            Look::active(enter, false)
        }
        (DeckView::Rating { title, .. }, Key::Title) => Look::active(title, false),
        (DeckView::Rating { labels, value, .. }, key) => match key.number() {
            Some(n) => Look::active(&labels[usize::from(n - 1)], *value == n),
            None => Look::off(),
        },
        _ => Look::off(),
    }
}

/// The images of `look` on `key`, per state (`None` for every state).
fn images(key: Key, look: &Look) -> Vec<(&'static str, Option<u16>)> {
    if !look.active {
        return vec![(BLACK, None)];
    }
    match key {
        Key::Enter => vec![(ORANGE, None)],
        Key::Title => vec![(BLACK, None)],
        _ => vec![(WHITE, Some(0)), (GREEN, Some(1))],
    }
}

struct Tracked {
    key: Key,
    /// What the key was last drawn as, so an unchanged key is not drawn again.
    drawn: Option<Look>,
}

#[derive(Default)]
struct Link {
    /// Present while connected to the app.
    outgoing: Option<mpsc::UnboundedSender<String>>,
    /// What the keys show, `Off` while not connected.
    shown: Shown,
    /// The visible instances bound to a footprints key. The runtime keeps their settings to
    /// itself, so they are recorded as they appear.
    keys: HashMap<InstanceId, Tracked>,
}

static LINK: LazyLock<Mutex<Link>> = LazyLock::new(|| Mutex::new(Link::default()));

async fn draw(instance: &Instance, tracked: &mut Tracked, look: Look) {
    if tracked.drawn.as_ref() == Some(&look) {
        return;
    }
    let id = &instance.instance_id;
    if let Err(e) = instance.set_title(Some(look.title.as_str()), None).await {
        log::warn!("Could not set the title of {id}: {e}");
    }
    for (image, state) in images(tracked.key, &look) {
        if let Err(e) = instance.set_image(Some(image), state).await {
            log::warn!("Could not set the image of {id}: {e}");
        }
    }
    let state = u16::from(look.lit);
    if let Err(e) = instance.set_state(state).await {
        log::warn!("Could not set the state of {id}: {e}");
    }
    instance.current_state_index.store(state, Ordering::Relaxed);
    tracked.drawn = Some(look);
}

/// Records which footprints key an instance is, or that it is none. Call it whenever the
/// instance appears or its settings change.
pub async fn track(instance: &Instance, key: Option<Key>) {
    log::info!(
        "{} appeared as footprints key {key:?}",
        instance.instance_id
    );
    let mut link = LINK.lock().await;
    let previous = link.keys.remove(&instance.instance_id);
    match key {
        Some(key) => {
            // A key that appears may show anything, it is always drawn. Without the app it does
            // nothing, so it is off.
            let mut tracked = Tracked { key, drawn: None };
            let look = look(&link.shown, key);
            draw(instance, &mut tracked, look).await;
            link.keys.insert(instance.instance_id.clone(), tracked);
        }
        // A key that stops being a footprints key does nothing any more, so it goes off. OpenDeck
        // has no way back to the title and image the user set, `setTitle` and `setImage`
        // overwrite them.
        None => {
            if let Some(mut previous) = previous {
                draw(instance, &mut previous, Look::off()).await;
            }
        }
    }
}

pub async fn untrack(instance: &Instance) {
    log::info!("{} disappeared", instance.instance_id);
    LINK.lock().await.keys.remove(&instance.instance_id);
}

/// Sends a press of `key` to the app. Returns false when it is not connected or speaks another
/// protocol.
pub async fn press(key: Key) -> bool {
    let link = LINK.lock().await;
    let Some(outgoing) = &link.outgoing else {
        return false;
    };
    if matches!(link.shown, Shown::Mismatch { .. }) {
        return false;
    }
    let message = serde_json::to_string(&PluginMessage::Press { key }).expect("a press serializes");
    outgoing.send(message).is_ok()
}

async fn show(shown: Shown) {
    let mut link = LINK.lock().await;
    for (id, tracked) in &mut link.keys {
        if let Some(instance) = get_instance(id.clone()).await {
            let look = look(&shown, tracked.key);
            draw(&instance, tracked, look).await;
        }
    }
    link.shown = shown;
}

/// Without the app the footprints keys do nothing, so they all go off.
async fn disconnected() {
    LINK.lock().await.outgoing = None;
    show(Shown::Off).await;
}

/// Keeps connecting to the app for as long as the plugin runs.
pub fn spawn() {
    tokio::spawn(async {
        let url = format!("ws://127.0.0.1:{PORT}");
        loop {
            // A failure means the app is simply not running, which is the normal case. Not worth
            // a log line every two seconds.
            if let Ok((socket, _)) = connect_async(url.as_str()).await {
                log::info!("Connected to footprints");
                session(socket).await;
                disconnected().await;
                log::info!("Disconnected from footprints");
            }
            tokio::time::sleep(RETRY).await;
        }
    });
}

async fn session(
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) {
    let (mut write, mut read) = socket.split();
    let (sender, mut presses) = mpsc::unbounded_channel();
    LINK.lock().await.outgoing = Some(sender);
    // Until the app says otherwise; footprints 0.3.0 never does.
    let mut app_protocol = UNANNOUNCED_PROTOCOL;
    loop {
        tokio::select! {
            Some(press) = presses.recv() => {
                if write.send(Message::text(press)).await.is_err() {
                    return;
                }
            }
            message = read.next() => match message {
                Some(Ok(Message::Text(text))) => match serde_json::from_str(text.as_str()) {
                    Ok(message) => {
                        if let AppMessage::Hello { protocol } = message {
                            log::info!("footprints speaks protocol {protocol}, this plugin {PROTOCOL}");
                        }
                        if let Some(shown) = receive(&mut app_protocol, message) {
                            show(shown).await;
                        }
                    }
                    // An app with another protocol may well send what this plugin cannot read.
                    Err(e) if app_protocol == PROTOCOL => {
                        log::warn!("Unknown message from footprints {text:?}: {e}")
                    }
                    Err(_) => {}
                },
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return,
                Some(Ok(_)) => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_protocol::Scale;

    #[test]
    fn keys_parse() {
        assert_eq!(parse_key(Some("3")), Some(Key::Three));
        assert_eq!(parse_key(Some(" Enter ")), Some(Key::Enter));
        assert_eq!(parse_key(Some("Title")), Some(Key::Title));
        assert_eq!(parse_key(Some("8")), None);
        assert_eq!(parse_key(Some("")), None);
        assert_eq!(parse_key(None), None);
    }

    fn rating(value: u8, labels: [&str; 7]) -> DeckView {
        DeckView::Rating {
            scale: Scale::Arousal,
            value,
            title: "Erregung".into(),
            labels: labels.map(String::from),
            enter: "Enter".into(),
        }
    }

    #[test]
    fn a_matching_app_is_shown() {
        let mut app_protocol = UNANNOUNCED_PROTOCOL;
        let hello = AppMessage::Hello { protocol: PROTOCOL };
        assert_eq!(receive(&mut app_protocol, hello), None);
        let state = AppMessage::State {
            state: DeckView::Default,
        };
        assert_eq!(
            receive(&mut app_protocol, state),
            Some(Shown::View(DeckView::Default))
        );
    }

    #[test]
    fn an_app_without_hello_speaks_the_first_protocol() {
        // footprints 0.3.0, which this plugin works with.
        assert_eq!(UNANNOUNCED_PROTOCOL, PROTOCOL);
        let mut app_protocol = UNANNOUNCED_PROTOCOL;
        let state = AppMessage::State {
            state: DeckView::Default,
        };
        assert_eq!(
            receive(&mut app_protocol, state),
            Some(Shown::View(DeckView::Default))
        );
    }

    #[test]
    fn an_app_with_another_protocol_is_never_shown() {
        let mut app_protocol = UNANNOUNCED_PROTOCOL;
        let hello = AppMessage::Hello {
            protocol: PROTOCOL + 1,
        };
        let mismatch = Shown::Mismatch { app: PROTOCOL + 1 };
        assert_eq!(receive(&mut app_protocol, hello), Some(mismatch.clone()));
        let state = AppMessage::State {
            state: DeckView::Default,
        };
        assert_eq!(receive(&mut app_protocol, state), Some(mismatch));
    }

    #[test]
    fn a_mismatch_says_what_to_update() {
        let newer = Shown::Mismatch { app: PROTOCOL + 1 };
        let older = Shown::Mismatch { app: PROTOCOL - 1 };
        assert_eq!(
            look(&newer, Key::Title),
            Look::active("Update\nplugin", false)
        );
        assert_eq!(
            look(&older, Key::Title),
            Look::active("Update\nfootprints", false)
        );
        for key in Key::VALUES.into_iter().chain([Key::Enter]) {
            assert_eq!(look(&newer, key), Look::off());
            assert_eq!(look(&older, key), Look::off());
        }
        assert_eq!(look(&Shown::Off, Key::Title), Look::off());
    }

    #[test]
    fn looks() {
        let confirm = Shown::View(DeckView::Confirm {
            enter: "Weiter".into(),
        });
        let rating = Shown::View(rating(
            4,
            ["Sehr\nniedrig", "b", "c", "d", "e", "f", "Sehr\nhoch"],
        ));
        let default = Shown::View(DeckView::Default);
        assert_eq!(look(&default, Key::Enter), Look::off());
        assert_eq!(look(&default, Key::Four), Look::off());
        assert_eq!(look(&default, Key::Title), Look::off());
        assert_eq!(look(&confirm, Key::Enter), Look::active("Weiter", false));
        assert_eq!(look(&confirm, Key::Four), Look::off());
        assert_eq!(look(&confirm, Key::Title), Look::off());
        assert_eq!(
            look(&rating, Key::One),
            Look::active("Sehr\nniedrig", false)
        );
        assert_eq!(look(&rating, Key::Four), Look::active("d", true));
        assert_eq!(look(&rating, Key::Seven), Look::active("Sehr\nhoch", false));
        assert_eq!(look(&rating, Key::Title), Look::active("Erregung", false));
        assert_eq!(look(&rating, Key::Enter), Look::active("Enter", false));
    }

    #[test]
    fn colours() {
        assert_eq!(images(Key::Three, &Look::off()), [(BLACK, None)]);
        assert_eq!(images(Key::Enter, &Look::off()), [(BLACK, None)]);
        assert_eq!(
            images(Key::Three, &Look::active("3", false)),
            [(WHITE, Some(0)), (GREEN, Some(1))]
        );
        assert_eq!(
            images(Key::Enter, &Look::active("Enter", false)),
            [(ORANGE, None)]
        );
        assert_eq!(
            images(Key::Title, &Look::active("Valenz", false)),
            [(BLACK, None)]
        );
    }
}
