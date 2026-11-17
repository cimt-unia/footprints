import { PersistedState } from "runed";
import type { TimeMod } from "./jitter.js";
import { NumIterations } from "./num_iter_state.js";

/** The resting state EEG recordings, one with the eyes open and one with them closed. */
export type EegKind = "eyes_open" | "eyes_closed";

export type BlockKind = "stimulus" | "neutral" | "pause" | "static_stimulus" | EegKind;

/**
 * One block of the experiment, modelled as a tagged union on `kind` so each kind can carry
 * only the fields it needs and future kinds can diverge in structure.
 *
 * - `stimulus`: every trial shows an emotional image.
 * - `static_stimulus`: every trial shows an emotional image without any walking, the image is
 *   followed directly by the rating.
 * - `neutral`: every trial shows a fixation cross in the image's place.
 * - `pause`: a break between blocks; no trials, it just waits for the experimenter to continue.
 * - `eyes_open` / `eyes_closed`: a single resting state EEG recording of `duration_s` seconds,
 *   with the eyes on a fixation cross or closed.
 *
 * `number | ""` because clearing the settings input persists an empty string, the same reason
 * the other numeric settings use this shape, see `jitter.ts`.
 */
export type Block =
	| TrialBlock
	| EegBlock
	| { kind: "pause" };

/** A block that runs trials, the only kind that has `settings`. */
export type TrialBlock = { kind: Exclude<BlockKind, "pause" | EegKind>; trials: number | ""; settings: TrialSettings };

export type EegBlock = { kind: EegKind; duration_s: number | "" };

export function is_trial_block(block: Block): block is TrialBlock {
	return block.kind !== "pause" && !is_eeg(block.kind);
}

export function is_eeg(kind: BlockKind): kind is EegKind {
	return kind === "eyes_open" || kind === "eyes_closed";
}

/** The recording length of an EEG block in seconds, an emptied input counts as 0. */
export function eeg_duration(block: EegBlock): number {
	return Math.max(0, Number(block.duration_s) || 0);
}

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
 * A single trial resolved from the block list. `block_nr` and `trial_in_block` are 0 based. `block`
 * is a copy, a running experiment must not see edits to the block list. A pause and an EEG
 * recording are a single entry each.
 */
export interface PlannedTrial {
	block_nr: number;
	trial_in_block: number;
	block: Block;
}

export function default_trial_settings(): TrialSettings {
	return {
		rating: { valence: true, arousal: true },
		speeds: { very_slow: false, slow: false, fast: false, very_fast: false },
		baseline: { duration: 2.5, jitter: 0.5 },
		stimulus: { duration: 2.5, jitter: 0.5 },
	};
}

/** The settings of a trial. Only the screens of trial blocks ask for them. */
export function trial_settings(trial: PlannedTrial): TrialSettings {
	if (!is_trial_block(trial.block)) {
		throw new Error(`A ${trial.block.kind} block has no trial settings`);
	}
	return trial.block.settings;
}

/** Narrows a plan entry to an EEG recording. Only the EEG phases ask for it. */
export function eeg_entry(trial: PlannedTrial): PlannedTrial & { block: EegBlock } {
	if (!is_eeg(trial.block.kind)) {
		throw new Error(`A ${trial.block.kind} block is no EEG recording`);
	}
	return trial as PlannedTrial & { block: EegBlock };
}

/** Whether trials of this kind show an emotional image, which is also what makes them rateable. */
export function shows_image(kind: BlockKind): boolean {
	return kind === "stimulus" || kind === "static_stimulus";
}

function copy<T>(value: T): T {
	return JSON.parse(JSON.stringify(value));
}

function copy_settings(settings: TrialSettings): TrialSettings {
	return copy(settings);
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
if (Blocks.current.some((block) => is_trial_block(block) && !block.settings)) {
	Blocks.current = Blocks.current.map((block) =>
		!is_trial_block(block) || block.settings ? block : { ...block, settings: legacy_settings() }
	);
}

/**
 * The settings a new block starts from: the last configured block's, since consecutive blocks
 * usually differ in one or two values at most.
 */
export function next_block_settings(blocks: Block[]): TrialSettings {
	const last = blocks.findLast(is_trial_block);
	return last ? copy_settings(last.settings) : default_trial_settings();
}

/** The scales at least one block asks for, which decides what the instructions have to explain. */
export function rated_scales(blocks: Block[]): RatingScales {
	const rated = (scale: keyof RatingScales) =>
		blocks.some((block) => is_trial_block(block) && shows_image(block.kind) && block.settings.rating[scale]);
	return { valence: rated("valence"), arousal: rated("arousal") };
}

/**
 * The block of a test run: a short walked stimulus block that practices every scale the real run
 * uses. Speeds and timing come from the first walked block, or the first block with trials.
 */
export function test_block(blocks: Block[]): Block {
	const trial_blocks = blocks.filter(is_trial_block);
	const source = trial_blocks.find((block) => block.kind === "stimulus") ?? trial_blocks[0];
	const settings = source ? copy_settings(source.settings) : default_trial_settings();
	return { kind: "stimulus", trials: 3, settings: { ...settings, rating: rated_scales(blocks) } };
}

export function block_trials(block: Block): number {
	if (block.kind === "pause") {
		return 0;
	}
	if (!is_trial_block(block)) {
		return 1;
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
 * contributes a single entry that stands in for the break, an EEG block one for its recording.
 */
export function build_trial_plan(blocks: Block[]): PlannedTrial[] {
	const plan: PlannedTrial[] = [];
	blocks.forEach((live, i) => {
		// One copy per block, shared by its trials, see `PlannedTrial`.
		const block = copy(live);
		if (!is_trial_block(block)) {
			plan.push({ block_nr: i, trial_in_block: 0, block });
			return;
		}
		const trials = block_trials(block);
		for (let trial = 0; trial < trials; trial++) {
			plan.push({ block_nr: i, trial_in_block: trial, block });
		}
	});
	return plan;
}
