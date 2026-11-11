# Stream Deck protocol

The app drives a Stream Deck through the OpenDeck plugin `com.cimtunia.resetbutton`
([streamdeck-resetbutton-plugin](https://git.rz.uni-augsburg.de/svki-a/streamdeck-resetbutton-plugin)).
The app decides what the deck shows; the plugin draws it and reports presses. Without the app
running, the plugin's buttons work on their own as before.

| | |
|---|---|
| server | the app, `ws://127.0.0.1:47823` |
| client | the plugin, reconnecting every 2 s while the app is not running |
| messages | one JSON object per text frame |

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

`state` carries what to show. It is sent right after the plugin connects and again whenever it
changes, never twice in a row unchanged.

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

## Setup

The app needs a single OpenDeck page for every state, scale and language: OpenDeck only lets its
own starterpack plugin switch profiles, so neither the app nor the plugin can pick one. Set the
**Footprints key** of its buttons:

| button | action | Footprints key |
|---|---|---|
| the scale's name | Reset State, no keystroke | `title` |
| the seven values | Toggle State | `1` to `7` |
| Enter | Reset State | `enter` |

Leave their titles empty: OpenDeck draws a title typed into a button over the one the plugin
sets. While the app runs, these buttons show what it sends and their presses go to the app
instead of typing their keystroke. Without the app they show `1` to `7` and `Enter` and type
their keystroke as before.
