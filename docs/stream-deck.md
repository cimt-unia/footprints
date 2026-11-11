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

## App to plugin

`state` carries the state to show. It is sent right after the plugin connects and again whenever
the state changes, never twice in a row unchanged.

```json
{"type":"state","state":{"kind":"default"}}
{"type":"state","state":{"kind":"confirm"}}
{"type":"state","state":{"kind":"rating","scale":"valence","value":4}}
```

`scale` is `valence` or `arousal`, `value` the selected rating, 1 to 7.

## Plugin to app

`press` reports a key press. `key` is `1` to `7` or `enter`.

```json
{"type":"press","key":"3"}
```

The app ignores a press the current state does not offer. An accepted one reaches the screens as
the key it stands for (`3` or `Enter`), exactly like typing it.

## Setup

In OpenDeck, set the **Footprints key** of each button of the rating profile: `1` to `7` on the
Toggle State buttons, `enter` on the Reset State button. While the app runs, those buttons show
what it sends and their presses go to the app instead of typing their keystroke.
