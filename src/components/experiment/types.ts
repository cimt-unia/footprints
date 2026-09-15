import type { PlannedTrial } from "$lib/blocks_state.js";
import type { Duration } from "$lib/durations.js";
import type { MarkerBlockType } from "$lib/lsl.js";
import type { MyEvents, MyStates } from "$lib/state_machine.js";
import { FiniteStateMachine } from "runed";

export interface ExperimentStateProps {
	/** The trial's walking condition. `kind` is what goes into the LsL marker, not `name`. */
	duration: Duration;
	state_machine: FiniteStateMachine<MyStates, MyEvents>;
	current_trial: PlannedTrial;
	/** Overrides the trial's own kind on markers a screen publishes itself, see `eventFromTrial`. */
	marker_block_type?: MarkerBlockType;
	img_valence?: "Low" | "High";
	img_arousal?: "Low" | "High";
	img_name?: string;
	img_url?: string;
	img_id?: number;
	/** The element flashed by the visual go cue, see `go_cue`. */
	cue_target?: HTMLElement;
}
