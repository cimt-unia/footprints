import { PersistedState } from "runed";
import type { TimeMod } from "./jitter.js";
import { NumIterations } from "./num_iter_state.js";

export type BlockKind = "stimulus" | "neutral" | "pause" | "static_stimulus";

/**
 * One block of the experiment, modelled as a tagged union on `kind` so each kind can carry
 * only the fields it needs and future kinds can diverge in structure.
 *
 * - `stimulus`: every trial shows an emotional image.
 * - `static_stimulus`: every trial shows an emotional image without any walking, the image is
 *   followed directly by the rating.
 * - `neutral`: every trial shows a fixation cross in the image's place.
 * - `pause`: a break between blocks; no trials, it just waits for the experimenter to continue.
 *
 * `number | ""` because clearing the settings input persists an empty string, the same reason
 * the other numeric settings use this shape, see `jitter.ts`.
 */
export type Block =
	| { kind: Exclude<BlockKind, "pause">; trials: number | ""; settings: TrialSettings }
	| { kind: "pause" };

/** Which of the two rating scales a block asks for. */
export interface RatingScales {
	valence: boolean;
	arousal: boolean;
}

/** The walking conditions a block adds on top of `normal`. */
export interface SpeedVariations {
	very_slow: boolean;
	slow: boolean;
	fast: boolean;
	very_fast: boolean;
}

/**
 * What a trial does, configured per block. Every trial kind carries all of it so switching a
 * block's kind loses nothing, a kind simply ignores what does not apply to it: `rating` on a
 * block without images and `speeds` on a block without walking.
 *
 * What holds for the whole session stays global: the calibrated walking speed, the track length,
 * the cues and the language.
 */
export interface TrialSettings {
	rating: RatingScales;
	speeds: SpeedVariations;
	baseline: TimeMod;
	stimulus: TimeMod;
}

/**
 * A single trial resolved from the block list. `block` and `trial_in_block` are 0 based. `settings`
 * is a copy, a running experiment must not see edits to the block list. A pause runs no trial and
 * holds the defaults as a placeholder.
 */
export interface PlannedTrial {
	block: number;
	trial_in_block: number;
	kind: BlockKind;
	settings: TrialSettings;
}

export function default_trial_settings(): TrialSettings {
	return {
		rating: { valence: true, arousal: true },
		speeds: { very_slow: false, slow: false, fast: false, very_fast: false },
		baseline: { duration: 2.5, jitter: 0.5 },
		stimulus: { duration: 2.5, jitter: 0.5 },
	};
}

/** Whether trials of this kind show an emotional image, which is also what makes them rateable. */
export function shows_image(kind: BlockKind): boolean {
	return kind === "stimulus" || kind === "static_stimulus";
}

function copy_settings(settings: TrialSettings): TrialSettings {
	return JSON.parse(JSON.stringify(settings));
}

function read_stored(key: string): any {
	try {
		return JSON.parse(localStorage.getItem(key) ?? "{}") ?? {};
	} catch {
		return {};
	}
}

// What these settings were set to before they moved onto the blocks. They are read from their old
// storage keys directly, the state objects that owned them no longer exist.
function legacy_settings(): TrialSettings {
	const defaults = default_trial_settings();
	const settings = read_stored("settings");
	return {
		rating: { ...defaults.rating, ...settings.rating },
		speeds: {
			very_slow: !!settings.very_slow,
			slow: !!settings.slow,
			fast: !!settings.fast,
			very_fast: !!settings.very_fast,
		},
		baseline: { ...defaults.baseline, ...read_stored("BaselineMod") },
		stimulus: { ...defaults.stimulus, ...read_stored("StimulusMod") },
	};
}

// Seeded from the previous flat trial count so existing installs start with a single stimulus
// block of the same length. This is its own storage key rather than a `Settings` field
// because `PersistedState` does not merge defaults into an already stored object.
export const Blocks = new PersistedState<Block[]>("blocks", [
	{ kind: "stimulus", trials: Number(NumIterations.current) || 2, settings: legacy_settings() },
]);

// A block stored before settings moved here has none. `PersistedState` hands it back as it is, so
// it is filled in once from the old global values and an existing setup keeps running the same.
if (Blocks.current.some((block) => block.kind !== "pause" && !block.settings)) {
	Blocks.current = Blocks.current.map((block) =>
		block.kind === "pause" || block.settings ? block : { ...block, settings: legacy_settings() }
	);
}

/**
 * The settings a new block starts from: the last configured block's, since consecutive blocks
 * usually differ in one or two values at most.
 */
export function next_block_settings(blocks: Block[]): TrialSettings {
	const last = blocks.findLast((block) => block.kind !== "pause");
	return last ? copy_settings(last.settings) : default_trial_settings();
}

/** The scales at least one block asks for, which decides what the instructions have to explain. */
export function rated_scales(blocks: Block[]): RatingScales {
	const rated = (scale: keyof RatingScales) =>
		blocks.some((block) => block.kind !== "pause" && shows_image(block.kind) && block.settings.rating[scale]);
	return { valence: rated("valence"), arousal: rated("arousal") };
}

/**
 * The block of a test run: a short walked stimulus block that practices every scale the real run
 * uses. Speeds and timing come from the first walked block, or the first block with trials.
 */
export function test_block(blocks: Block[]): Block {
	const trial_blocks = blocks.filter((block) => block.kind !== "pause");
	const source = trial_blocks.find((block) => block.kind === "stimulus") ?? trial_blocks[0];
	const settings = source ? copy_settings(source.settings) : default_trial_settings();
	return { kind: "stimulus", trials: 3, settings: { ...settings, rating: rated_scales(blocks) } };
}

export function block_trials(block: Block): number {
	if (block.kind === "pause") {
		return 0;
	}
	return Math.max(0, Math.floor(Number(block.trials) || 0));
}

export function total_trials(blocks: Block[]): number {
	return blocks.reduce((sum, block) => sum + block_trials(block), 0);
}

/**
 * Flattens the block list into one entry per trial. The index into this array is the global
 * trial counter (`ExperimentIteration`), which never restarts at a block boundary. That is
 * what keeps the image sampling without replacement running across blocks. A pause block
 * contributes a single entry that stands in for the break.
 */
export function build_trial_plan(blocks: Block[]): PlannedTrial[] {
	const plan: PlannedTrial[] = [];
	blocks.forEach((block, i) => {
		if (block.kind === "pause") {
			plan.push({ block: i, trial_in_block: 0, kind: "pause", settings: default_trial_settings() });
			return;
		}
		const settings = copy_settings(block.settings);
		const trials = block_trials(block);
		for (let trial = 0; trial < trials; trial++) {
			plan.push({ block: i, trial_in_block: trial, kind: block.kind, settings });
		}
	});
	return plan;
}
