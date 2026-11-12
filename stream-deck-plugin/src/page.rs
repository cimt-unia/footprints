//! Creates the Footprints page in OpenDeck, so a deck needs no setup by hand.
//!
//! OpenDeck has no way for a plugin to bring a profile along. It finds profiles by reading its
//! profile folder, though, and loads one only when it is selected or its device connects. So the
//! plugin writes `profiles/<device>/Footprints.json` for every deck it sees, unless the file
//! already exists: a page changed by hand is never overwritten.
//!
//! The file is in OpenDeck's own format, which is not a public interface. It is written the way
//! OpenDeck 2.14 stores a profile, including a copy of each action's definition, which OpenDeck
//! does not refresh from the manifest when it loads the page.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use deck_protocol::Key;
use serde_json::{Value, json};

const PROFILE: &str = "Footprints";

/// OpenDeck's config folder. OpenDeck starts a plugin inside its folder,
/// `<config>/plugins/<plugin>.sdPlugin`, on every platform.
fn config_dir_of(plugin_dir: &Path) -> Option<PathBuf> {
    let plugins = plugin_dir.parent()?;
    let is_plugin_dir = plugin_dir
        .file_name()?
        .to_str()?
        .ends_with(".sdPlugin");
    (is_plugin_dir && plugins.file_name()? == "plugins").then(|| plugins.parent().map(Path::to_owned))?
}

/// Where the footprints keys go on a deck, as key positions counted row by row.
#[derive(Debug, PartialEq, Eq)]
struct Layout {
    keys: usize,
    title: usize,
    values: [usize; 7],
    enter: usize,
}

/// The layout for a deck of `rows` × `columns` keys, or `None` if it has too few keys for the
/// scale's name, seven values and Enter.
fn layout(rows: u8, columns: u8) -> Option<Layout> {
    let (rows, columns) = (usize::from(rows), usize::from(columns));
    let keys = rows * columns;
    if keys < 9 {
        return None;
    }
    // On a deck seven keys wide the values fill the second row under the scale's name, which is
    // the layout the Stream Deck XL was set up with. A smaller deck takes them in a row.
    let (title, first_value) = if columns >= 7 && rows >= 3 {
        ((columns - 1) / 2, columns)
    } else {
        (0, 1)
    };
    Some(Layout {
        keys,
        title,
        values: std::array::from_fn(|n| first_value + n),
        enter: keys - 1,
    })
}

/// A key state as OpenDeck stores it.
fn state(image: &str, colour: &str, text: &str, style: &str, stroke: u8) -> Value {
    json!({
        "alignment": "middle",
        "background_colour": "#000000",
        "colour": colour,
        "family": "Liberation Sans",
        "image": image,
        "image_override": null,
        "image_scale": 100,
        "name": "",
        "show": true,
        "size": 16,
        "stroke_colour": "#000000",
        "stroke_size": stroke,
        "style": style,
        "text": text,
        "text_override": null,
        "underline": false,
    })
}

/// The definition of one of this plugin's actions as OpenDeck copies it into a profile, from
/// `manifest.json`.
fn action(plugin: &str, uuid: &str, name: &str, icon: &str, tooltip: &str, states: usize, automatic_states: bool) -> Value {
    let icon = format!("plugins/{plugin}/icons/{icon}.svg");
    json!({
        "controllers": ["Keypad"],
        "disable_automatic_states": !automatic_states,
        "encoder": null,
        "icon": icon,
        "name": name,
        "plugin": plugin,
        "property_inspector": format!("plugins/{plugin}/property-inspector/footprints.html"),
        "states": (0..states).map(|_| state(&icon, "#FFFFFF", "", "Regular", 3)).collect::<Vec<_>>(),
        "supported_in_multi_actions": true,
        "tooltip": tooltip,
        "uuid": uuid,
        "visible_in_action_list": true,
    })
}

fn key_action(plugin: &str) -> Value {
    action(
        plugin,
        crate::KEY_ACTION,
        "Key",
        "key",
        "A footprints key: the scale's name or Enter.",
        1,
        true,
    )
}

fn value_action(plugin: &str) -> Value {
    action(
        plugin,
        crate::VALUE_ACTION,
        "Value",
        "value",
        "A footprints key that can be lit: one of the seven values.",
        2,
        false,
    )
}

/// A key bound to `footprints`. Its titles are empty, OpenDeck would draw a typed title over the
/// one the plugin sets. The plugin draws its colours as soon as it appears.
fn key(action: &Value, position: usize, footprints: Key, text_colour: &str) -> Value {
    let image = action["icon"].as_str().unwrap_or_default();
    let states = action["states"]
        .as_array()
        .map(|states| states.iter().map(|_| state(image, text_colour, "", "Bold", 0)).collect::<Vec<_>>())
        .unwrap_or_default();
    json!({
        "action": action,
        "children": null,
        "context": format!("Keypad.{position}.0"),
        "current_state": 0,
        "settings": { "footprints": footprints.as_str() },
        "states": states,
    })
}

/// The Footprints page for `layout`, as OpenDeck stores a profile.
fn page(plugin: &str, layout: &Layout) -> Value {
    let key_action = key_action(plugin);
    let value_action = value_action(plugin);
    let mut keys = vec![Value::Null; layout.keys];
    keys[layout.title] = key(&key_action, layout.title, Key::Title, "#ffffff");
    for (&position, value) in layout.values.iter().zip(Key::VALUES) {
        keys[position] = key(&value_action, position, value, "#000000");
    }
    keys[layout.enter] = key(&key_action, layout.enter, Key::Enter, "#000000");
    json!({ "keys": keys, "sliders": [], "infobars": [] })
}

/// Writes the Footprints page for a deck into `config`, unless it has one. Returns whether it
/// wrote it.
fn ensure_in(config: &Path, plugin: &str, device: &str, rows: u8, columns: u8) -> io::Result<bool> {
    // The device id names a folder; anything that could leave the profile folder is no device.
    if device.is_empty() || device.contains(['/', '\\']) || device.starts_with('.') {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("unusable device id {device:?}")));
    }
    let Some(layout) = layout(rows, columns) else {
        return Ok(false);
    };
    let dir = config.join("profiles").join(device);
    let path = dir.join(format!("{PROFILE}.json"));
    if path.exists() {
        return Ok(false);
    }
    fs::create_dir_all(&dir)?;
    // Written next to it first, under a name OpenDeck does not take for a profile, so OpenDeck
    // never reads half a page.
    let partial = dir.join(format!("{PROFILE}.json.partial"));
    fs::write(&partial, serde_json::to_vec(&page(plugin, &layout))?)?;
    fs::rename(&partial, &path)?;
    Ok(true)
}

/// Makes sure a deck has the Footprints page. Failures are logged, the plugin works without it.
pub fn ensure(device: &str, rows: u8, columns: u8) {
    let Some(plugin_dir) = std::env::current_dir().ok() else {
        log::warn!("Cannot create the Footprints page: no working directory");
        return;
    };
    let (Some(config), Some(plugin)) = (
        config_dir_of(&plugin_dir),
        plugin_dir.file_name().and_then(|name| name.to_str()),
    ) else {
        log::warn!("Cannot create the Footprints page: {} is not an OpenDeck plugin folder", plugin_dir.display());
        return;
    };
    match ensure_in(&config, plugin, device, rows, columns) {
        Ok(true) => log::info!("Created the Footprints page for {device}, select it in OpenDeck"),
        Ok(false) if layout(rows, columns).is_none() => {
            log::info!("{device} has too few keys for the Footprints page")
        }
        Ok(false) => {}
        Err(e) => log::warn!("Cannot create the Footprints page for {device}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLUGIN: &str = "com.cimtunia.footprints-deck.sdPlugin";

    #[test]
    fn config_dir_is_two_levels_up() {
        let plugin = Path::new("/home/u/.config/opendeck/plugins").join(PLUGIN);
        assert_eq!(config_dir_of(&plugin), Some(PathBuf::from("/home/u/.config/opendeck")));
        assert_eq!(config_dir_of(Path::new("/home/u/src/footprints-deck")), None);
        assert_eq!(config_dir_of(&Path::new("/home/u/other").join(PLUGIN)), None);
    }

    #[test]
    fn xl_layout_is_the_one_it_was_set_up_with() {
        assert_eq!(
            layout(4, 8),
            Some(Layout { keys: 32, title: 3, values: [8, 9, 10, 11, 12, 13, 14], enter: 31 })
        );
    }

    #[test]
    fn smaller_decks_take_the_values_in_a_row() {
        assert_eq!(
            layout(3, 5),
            Some(Layout { keys: 15, title: 0, values: [1, 2, 3, 4, 5, 6, 7], enter: 14 })
        );
        assert_eq!(layout(2, 3), None);
        assert_eq!(layout(2, 4), None);
    }

    #[test]
    fn page_binds_every_footprints_key() {
        let page = page(PLUGIN, &layout(4, 8).unwrap());
        let keys = page["keys"].as_array().unwrap();
        assert_eq!(keys.len(), 32);
        let bound: Vec<(usize, &str, &str)> = keys
            .iter()
            .enumerate()
            .filter(|(_, k)| !k.is_null())
            .map(|(i, k)| (i, k["action"]["uuid"].as_str().unwrap(), k["settings"]["footprints"].as_str().unwrap()))
            .collect();
        assert_eq!(bound[0], (3, crate::KEY_ACTION, "title"));
        assert_eq!(bound[1], (8, crate::VALUE_ACTION, "1"));
        assert_eq!(bound[7], (14, crate::VALUE_ACTION, "7"));
        assert_eq!(bound[8], (31, crate::KEY_ACTION, "enter"));
        assert_eq!(bound.len(), 9);
        // A value has both of its states, every title is empty.
        assert_eq!(keys[8]["states"].as_array().unwrap().len(), 2);
        assert!(keys.iter().flat_map(|k| k["states"].as_array().into_iter().flatten()).all(|s| s["text"] == ""));
        assert_eq!(keys[8]["context"], "Keypad.8.0");
    }

    #[test]
    fn writes_the_page_once_and_never_over_a_changed_one() {
        let config = std::env::temp_dir().join(format!("footprints-page-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&config);
        assert!(ensure_in(&config, PLUGIN, "sd-TEST", 4, 8).unwrap());
        let path = config.join("profiles/sd-TEST/Footprints.json");
        let written: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(written["keys"][31]["settings"]["footprints"], "enter");
        fs::write(&path, "changed by hand").unwrap();
        assert!(!ensure_in(&config, PLUGIN, "sd-TEST", 4, 8).unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "changed by hand");
        assert!(!config.join("profiles/sd-TEST/Footprints.json.partial").exists());
        assert!(ensure_in(&config, PLUGIN, "../escape", 4, 8).is_err());
        assert!(!ensure_in(&config, PLUGIN, "sd-MINI", 2, 3).unwrap());
        fs::remove_dir_all(&config).unwrap();
    }
}
