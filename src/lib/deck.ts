import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * What the Stream Deck shows and which of its keys do anything, see `deck.rs`. Each phase of the
 * state machine declares one, the deck follows the run without the screens knowing about it.
 */
export type DeckState =
	| { kind: "default" }
	| { kind: "confirm" }
	| { kind: "rating"; scale: "valence" | "arousal"; value: number };

export const DECK_DEFAULT: DeckState = { kind: "default" };

/** A run's hold on the deck. */
export interface DeckClaim {
	set(state: DeckState): void;
	/** Hands the deck back to the claim below. Safe to call more than once. */
	release(): void;
}

// A test run is started from inside the instructions, so two runs can be alive at once. The
// newest claim owns the deck; when it is released the one below takes over again, whichever
// order the two runs' teardowns happen in.
const claims: { state: DeckState }[] = [];
let shown = JSON.stringify(DECK_DEFAULT);
let unlisten: Promise<UnlistenFn> | undefined;

function sync() {
	const state = claims.at(-1)?.state ?? DECK_DEFAULT;
	const json = JSON.stringify(state);
	if (json === shown) {
		return;
	}
	shown = json;
	invoke("set_deck_state", { state }).catch((e) =>
		console.error("Could not update the Stream Deck", e),
	);
}

// A deck press arrives as the key it stands for and is replayed as a key press, so every
// screen handles it like typing. The backend only forwards keys the current state offers.
function forward_presses() {
	unlisten = listen<string>("deck-key", (e) => {
		window.dispatchEvent(new KeyboardEvent("keydown", { key: e.payload }));
	});
}

export function claim_deck(state: DeckState = DECK_DEFAULT): DeckClaim {
	const claim = { state };
	claims.push(claim);
	if (claims.length === 1) {
		forward_presses();
	}
	sync();
	return {
		set(next) {
			claim.state = next;
			sync();
		},
		release() {
			const index = claims.indexOf(claim);
			if (index === -1) {
				return;
			}
			claims.splice(index, 1);
			if (claims.length === 0) {
				void unlisten?.then((f) => f());
				unlisten = undefined;
			}
			sync();
		},
	};
}
