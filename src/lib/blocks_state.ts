import { PersistedState } from "runed";
import { NumIterations } from "./num_iter_state.js";

export type BlockKind = "stimulus" | "neutral" | "pause";

/**
 * One block of the experiment, modelled as a tagged union on `kind` so each kind can carry
 * only the fields it needs and future kinds can diverge in structure.
 *
 * - `stimulus`: every trial shows an emotional image.
 * - `neutral`: every trial shows a fixation cross in the image's place.
 * - `pause`: a break between blocks; no trials, it just waits for the experimenter to continue.
 *
 * `number | ""` because clearing the settings input persists an empty string, the same reason
 * the other numeric settings use this shape, see `jitter.ts`.
 */
export type Block =
	| { kind: "stimulus"; trials: number | "" }
	| { kind: "neutral"; trials: number | "" }
	| { kind: "pause" };

/** A single trial resolved from the block list. `block` and `trial_in_block` are 0 based. */
export interface PlannedTrial {
	block: number;
	trial_in_block: number;
	kind: BlockKind;
}

// Seeded from the previous flat trial count so existing installs start with a single stimulus
// block of the same length. This is its own storage key rather than a `Settings` field
// because `PersistedState` does not merge defaults into an already stored object.
export const Blocks = new PersistedState<Block[]>("blocks", [
	{ kind: "stimulus", trials: Number(NumIterations.current) || 2 },
]);

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
			plan.push({ block: i, trial_in_block: 0, kind: "pause" });
			return;
		}
		const trials = block_trials(block);
		for (let trial = 0; trial < trials; trial++) {
			plan.push({ block: i, trial_in_block: trial, kind: block.kind });
		}
	});
	return plan;
}
