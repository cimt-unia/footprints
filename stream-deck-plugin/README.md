# Footprints Deck

OpenDeck/OpenAction plugin that turns a Stream Deck into the input device of the
[footprints](https://github.com/cimt-unia/footprints) app, version
**0.3.0 or later**. Earlier versions of footprints are not supported.

## Install

1. Download `footprints-deck.zip` from the footprints release you use.
2. In OpenDeck, open **Plugins** and choose **Install from file**, then pick
   the zip. Installing a newer zip replaces the old version.
3. Select the **Footprints** profile the plugin creates for your deck (see
   [below](#the-footprints-page)).

The zip has binaries for Linux (x86_64), Windows (x86_64) and macOS (Apple
silicon and Intel).

The plugin has the version of the footprints release it comes with. If the
scale's name key says **Update plugin** or **Update footprints**, the two speak
different protocols: install the plugin from the release of the app you run.

## Building

```bash
scripts/package.sh
```

builds the plugin for this machine and packs it into
`target/footprints-deck-<version>.zip`, ready for **Install from file**. The
release workflow (`.github/workflows/publish.yml`) builds every platform and
packs them with the same script. The zip's manifest gets the version of
`Cargo.toml`, which `scripts/bump-version.sh` in footprints sets along with the
app's.

## How it works

While the footprints app runs, the plugin stays connected to it
(`ws://127.0.0.1:47823`, retrying every 2 s) and the keys bound to it show what
the app sends, with every text in the study's language: blank while there is
nothing to press, only Enter when it waits for a confirmation, the scale's name
and anchors with the current rating lit while rating. Presses go to the app.
While the app is not running, the keys are black and do nothing. The protocol
is described in footprints' `docs/stream-deck.md`; its messages are the
`deck-protocol` crate next to this one, which the app builds against too.

Each key has a **Footprints key** setting: `1` to `7`, `enter` or `title` (the
key naming the scale). A key without one does nothing. Leave the titles of the
keys empty, OpenDeck draws a typed title over the plugin's.

The two actions differ only in their states: the seven values use **Value**,
whose second state shows a selected value; the scale's name and Enter use
**Key**.

## The Footprints page

The plugin sets up the page itself: for every connected deck it creates an
OpenDeck profile named **Footprints** with the scale's name, the seven values
and Enter, their Footprints keys set and their titles empty. On a deck seven
keys wide or more (Stream Deck XL) the values fill the second row under the
name and Enter is the last key; smaller decks get them in a row. A deck with
fewer than nine keys gets no page. An existing Footprints profile is never
overwritten, so changes made by hand stay. The profile is written in OpenDeck's
own file format, as OpenDeck 2.14 stores it.
