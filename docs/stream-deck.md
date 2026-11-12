# Stream Deck protocol

The app drives a Stream Deck through the OpenDeck plugin Footprints Deck,
`com.cimtunia.footprints-deck`, in `stream-deck-plugin/`. Each release carries it as
`footprints-deck.zip`, which OpenDeck installs with **Plugins → Install from file**.
The app decides what the deck shows; the plugin draws it and reports presses. Without the app
running, the buttons bound to it are black and do nothing.

| | |
|---|---|
| server | the app, `ws://127.0.0.1:47823` |
| client | the plugin, reconnecting every 2 s while the app is not running |
| messages | one JSON object per text frame |

Both sides build against the `deck-protocol` crate, which defines every message below, so they
cannot disagree on the format. Its tests check the examples on this page; change them together.

## Deck states

What the deck shows follows the phase of the run, see `phases` in `state_machine.svelte.ts`.

| state | shown during | keys that do anything |
|---|---|---|
| `default` | baseline, stimulus, go, outside a run | none |
| `confirm` | instructions, pause, confirm | `enter` |
| `rating` | rating | `1` to `7`, `enter` |

The frontend only reports the state, `set_deck_state` in `deck.rs`. The backend fills in the
texts in the language of the study; they live in the `deck` section of `src/lib/lang/*.json`
next to the rest of the study's texts.

## App to plugin

`hello` comes first on every connection and names the protocol the app speaks, `PROTOCOL` in
`deck-protocol`. Its shape never changes, so any plugin can read it.

```json
{"type":"hello","protocol":1}
```

`state` carries what to show. It is sent right after the hello and again whenever it changes,
never twice in a row unchanged.

```json
{"type":"state","state":{"kind":"default"}}
{"type":"state","state":{"kind":"confirm","enter":"Enter"}}
{"type":"state","state":{"kind":"rating","scale":"valence","value":4,"title":"Valenz",
  "labels":["Sehr\nnegativ","Mäßig\nnegativ","Etwas\nnegativ","Neutral",
            "Etwas\npositiv","Mäßig\npositiv","Sehr\npositiv"],
  "enter":"Enter"}}
```

`scale` is `valence` or `arousal`, `value` the selected rating, 1 to 7. `title` names the scale,
`labels` are the anchors of the values 1 to 7, `enter` labels the Enter key. The plugin draws the
texts as they are and lights the key of `value`; in `default` every key is blank, in `confirm`
every key but Enter.

## Plugin to app

`press` reports a key press. `key` is `1` to `7`, `enter` or `title`.

```json
{"type":"press","key":"3"}
```

The app ignores a press the current state does not offer. An accepted one reaches the screens as
the key it stands for (`3` or `Enter`), exactly like typing it.

## Versions

The app and the plugin are released together and share the release version, set with
`scripts/bump-version.sh` and checked by the release workflow.

The protocol has a version of its own, `PROTOCOL` in `deck-protocol`. It goes up only with a
change a plugin built against the previous one cannot handle, such as a new state kind or a
renamed field; a field old plugins may ignore needs none. A plugin that gets a `hello` with
another protocol keeps every key off, ignores presses, and has the scale's name say
**Update plugin** (the app's protocol is newer) or **Update footprints** (it is older).
footprints 0.3.0 sends no `hello`; it speaks protocol 1.

## Setup

The app needs a single OpenDeck page for every state, scale and language: OpenDeck only lets its
own starterpack plugin switch profiles, so neither the app nor the plugin can pick one. The plugin
creates that page itself, an OpenDeck profile named **Footprints** for every connected deck; select
it in OpenDeck. Set up by hand, its buttons have this **Footprints key**:

| button | action | Footprints key |
|---|---|---|
| the scale's name | Key | `title` |
| the seven values | Value | `1` to `7` |
| Enter | Key | `enter` |

Leave their titles empty: OpenDeck draws a title typed into a button over the one the plugin
sets. While the app runs, these buttons show what it sends and their presses go to the app.
Without the app they are black and do nothing; a button without a Footprints key does nothing
either.
