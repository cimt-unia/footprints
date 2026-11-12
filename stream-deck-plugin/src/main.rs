use deck_protocol::Key;
use openaction::*;
use serde::{Deserialize, Serialize};

mod footprints;
mod page;

// Both actions do the same, they differ in their states: a value has a second state, which
// footprints lights the selected value with. The scale's name and Enter are plain keys.
const KEY_ACTION: &str = "com.cimtunia.footprints-deck.key";
const VALUE_ACTION: &str = "com.cimtunia.footprints-deck.value";

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default)]
struct KeySettings {
    /// The footprints key this button stands for, `1` to `7`, `enter` or `title`, see
    /// `footprints.rs`. A button without one does nothing.
    footprints: Option<String>,
}

impl KeySettings {
    fn footprints_key(&self) -> Option<Key> {
        footprints::parse_key(self.footprints.as_deref())
    }
}

/// Hands a press to footprints if the button is one of its keys.
async fn pressed(settings: &KeySettings) {
    let Some(key) = settings.footprints_key() else {
        log::info!("Press of a button without a footprints key ignored");
        return;
    };
    if footprints::press(key).await {
        log::info!("Press of footprints key {key} sent to footprints");
    } else {
        log::info!(
            "Press of footprints key {key} ignored, footprints is not connected or speaks another protocol"
        );
    }
}

struct ValueAction;

#[async_trait]
impl Action for ValueAction {
    const UUID: &'static str = VALUE_ACTION;
    type Settings = KeySettings;

    async fn will_appear(&self, instance: &Instance, settings: &KeySettings) -> OpenActionResult<()> {
        footprints::track(instance, settings.footprints_key()).await;
        Ok(())
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &KeySettings,
    ) -> OpenActionResult<()> {
        footprints::track(instance, settings.footprints_key()).await;
        Ok(())
    }

    async fn will_disappear(&self, instance: &Instance, _: &KeySettings) -> OpenActionResult<()> {
        footprints::untrack(instance).await;
        Ok(())
    }

    async fn key_down(&self, _: &Instance, settings: &KeySettings) -> OpenActionResult<()> {
        pressed(settings).await;
        Ok(())
    }
}

struct KeyAction;

#[async_trait]
impl Action for KeyAction {
    const UUID: &'static str = KEY_ACTION;
    type Settings = KeySettings;

    async fn will_appear(&self, instance: &Instance, settings: &KeySettings) -> OpenActionResult<()> {
        footprints::track(instance, settings.footprints_key()).await;
        Ok(())
    }

    async fn did_receive_settings(
        &self,
        instance: &Instance,
        settings: &KeySettings,
    ) -> OpenActionResult<()> {
        footprints::track(instance, settings.footprints_key()).await;
        Ok(())
    }

    async fn will_disappear(&self, instance: &Instance, _: &KeySettings) -> OpenActionResult<()> {
        footprints::untrack(instance).await;
        Ok(())
    }

    async fn key_down(&self, _: &Instance, settings: &KeySettings) -> OpenActionResult<()> {
        pressed(settings).await;
        Ok(())
    }
}

/// Gives every deck the Footprints page, those connected at start and those plugged in later.
struct Pages;

#[async_trait]
impl global_events::GlobalEventHandler for Pages {
    async fn plugin_ready(&self) -> OpenActionResult<()> {
        for device in get_connected_devices().await.values() {
            page::ensure(&device.id, device.size.rows, device.size.columns);
        }
        Ok(())
    }

    async fn device_did_connect(
        &self,
        event: global_events::DeviceDidConnectEvent,
    ) -> OpenActionResult<()> {
        let size = event.deviceInfo.size;
        page::ensure(&event.device, size.rows, size.columns);
        Ok(())
    }
}

static PAGES: Pages = Pages;

#[tokio::main]
async fn main() -> OpenActionResult<()> {
    if let Err(error) = simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Stdout,
        simplelog::ColorChoice::Never,
    ) {
        eprintln!("Logger initialization failed: {error}");
    }

    footprints::spawn();
    global_events::set_global_event_handler(&PAGES);
    register_action(KeyAction).await;
    register_action(ValueAction).await;
    run(std::env::args().collect()).await
}
