// The timers live outside the state machine, which only reacts to the events they send. A timer
// that outlives its phase, e.g. after a cancel, sends an event the machine then ignores.

import { useDebounce } from "runed";
import type { ExperimentMachine } from "./state_machine.svelte.js";

export const baseline_debounce = (sm: ExperimentMachine, delay: number) => {
	console.log(delay);
	useDebounce(
		() => {
			sm.send("start");
		},
		() => delay
	)();
}

export const stimulus_debounce = (sm: ExperimentMachine, delay: number) => {
	console.log(delay);
	useDebounce(
		() => {
			sm.send("s_fin");
		},
		() => delay
	)();
}

