import type { PlannedTrial } from "$lib/blocks_state.js";
import type { Duration } from "$lib/durations.js";
import type { MarkerRunKind } from "$lib/lsl.js";
import type { InstructionSet } from "$lib/instructions.js";
import type { ExperimentMachine } from "$lib/state_machine.svelte.js";

export interface ExperimentStateProps {
	/** The trial's walking condition. `kind` is what goes into the LsL marker, not `name`. */
	duration: Duration;
	state_machine: ExperimentMachine;
	current_trial: PlannedTrial;
	/** Overrides the trial's own kind on markers a screen publishes itself, see `eventFromTrial`. */
	marker_type?: MarkerRunKind;
	img_valence?: "Low" | "High";
	img_arousal?: "Low" | "High";
	img_name?: string;
	img_url?: string;
	img_id?: number;
	/** The element flashed by the visual go cue, see `go_cue`. */
	cue_target?: HTMLElement;
	/** The pages the instruction screen shows. Only an experiment has them, a test run does not. */
	instructions?: InstructionSet;
}
