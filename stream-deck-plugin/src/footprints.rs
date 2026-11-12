//! Lets the footprints app (0.3.0 or later) drive the keys.
//!
//! The app serves a WebSocket on `127.0.0.1`. This module stays connected to it, retrying while
//! the app is not running. Keys whose `footprints` setting names a footprints key (`1` to `7`,
//! `enter` or `title`) show what the app sends, and their presses go to the app. Every text comes
//! from the app, in the language of the study, so one page serves both scales and every language.
//! Without the app they are black and do nothing.
//!
//! The protocol is documented in footprints' `docs/stream-deck.md`, its messages are the
//! `deck-protocol` crate the app builds against too.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::Ordering;
use std::time::Duration;

use deck_protocol::{AppMessage, DeckView, Key, PORT, PluginMessage};
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

/// What `key` shows in `view`.
fn look(view: &DeckView, key: Key) -> Look {
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
    /// The last view the app sent, valid while connected.
    view: Option<DeckView>,
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
    log::info!("{} appeared as footprints key {key:?}", instance.instance_id);
    let mut link = LINK.lock().await;
    let previous = link.keys.remove(&instance.instance_id);
    let view = link.view.clone();
    match key {
        Some(key) => {
            // A key that appears may show anything, it is always drawn. Without the app it does
            // nothing, so it is off.
            let mut tracked = Tracked { key, drawn: None };
            let look = match &view {
                Some(view) => look(view, key),
                None => Look::off(),
            };
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

/// Sends a press of `key` to the app. Returns false when it is not connected.
pub async fn press(key: Key) -> bool {
    let link = LINK.lock().await;
    let Some(outgoing) = &link.outgoing else {
        return false;
    };
    let message =
        serde_json::to_string(&PluginMessage::Press { key }).expect("a press serializes");
    outgoing.send(message).is_ok()
}

async fn apply(view: DeckView) {
    let mut link = LINK.lock().await;
    for (id, tracked) in &mut link.keys {
        if let Some(instance) = get_instance(id.clone()).await {
            let look = look(&view, tracked.key);
            draw(&instance, tracked, look).await;
        }
    }
    link.view = Some(view);
}

/// Without the app the footprints keys do nothing, so they all go off.
async fn disconnected() {
    let mut link = LINK.lock().await;
    link.outgoing = None;
    link.view = None;
    for (id, tracked) in &mut link.keys {
        if let Some(instance) = get_instance(id.clone()).await {
            draw(&instance, tracked, Look::off()).await;
        }
    }
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
    loop {
        tokio::select! {
            Some(press) = presses.recv() => {
                if write.send(Message::text(press)).await.is_err() {
                    return;
                }
            }
            message = read.next() => match message {
                Some(Ok(Message::Text(text))) => match serde_json::from_str(text.as_str()) {
                    Ok(AppMessage::State { state }) => apply(state).await,
                    Err(e) => log::warn!("Unknown message from footprints {text:?}: {e}"),
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
    fn looks() {
        let confirm = DeckView::Confirm {
            enter: "Weiter".into(),
        };
        let rating = rating(4, ["Sehr\nniedrig", "b", "c", "d", "e", "f", "Sehr\nhoch"]);
        assert_eq!(look(&DeckView::Default, Key::Enter), Look::off());
        assert_eq!(look(&DeckView::Default, Key::Four), Look::off());
        assert_eq!(look(&DeckView::Default, Key::Title), Look::off());
        assert_eq!(look(&confirm, Key::Enter), Look::active("Weiter", false));
        assert_eq!(look(&confirm, Key::Four), Look::off());
        assert_eq!(look(&confirm, Key::Title), Look::off());
        assert_eq!(look(&rating, Key::One), Look::active("Sehr\nniedrig", false));
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
        assert_eq!(images(Key::Enter, &Look::active("Enter", false)), [(ORANGE, None)]);
        assert_eq!(images(Key::Title, &Look::active("Valenz", false)), [(BLACK, None)]);
    }
}
