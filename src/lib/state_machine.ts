import { FiniteStateMachine, PersistedState } from "runed";
import { eventFromTrial, pauseMarker, publish_event, sessionMarker } from "./lsl.js";
import { invoke } from "@tauri-apps/api/core";
import { stimulus_debounce } from "./debounce.js";
import { StimulusModState } from "./stimulus_time_mod.js";
import { jittered_duration } from "./jitter.js";
import type { PlannedTrial } from "./blocks_state.js";
import { Settings } from "./settings_state.js";
import type { Duration } from "./durations.js";
import { SpeedState } from "./speed_state.js";
import { LengthState } from "./length_state.js";
import type { TrialImages } from "./trial_images.svelte.js";
export type MyStates = "instruction" | "baseline" | "stimulus" | "go" | "rating" | "confirm" | "pause" | "canceled";
export type MyEvents = "start" | "s_fin" | "g_fin" | "rated" | "confirmed" | "continued" | "cancel" | "instructed" | "test_finished";

// global reactive variable to track the experiment progress
export const ExperimentIteration = new PersistedState("ex_iter", 0);

interface Log {
	baseline_time: string,
	stimulus_time: string,
	go_time: string,
	rating_time: string,
}

/**
 * @param mode A test run skips the instructions and stamps every marker as `test`, so a
 * practice run cannot be mistaken for real data in the recording.
 */
export function create_state_machine(cancel_callback: () => void, plan: PlannedTrial[], durations: Duration[], images: TrialImages, mode: "experiment" | "test" = "experiment"): FiniteStateMachine<MyStates, MyEvents> {
	const marker_type = mode === "test" ? "test" as const : undefined;
	// We just created the experiment state machine so we are in the first iteration.
	ExperimentIteration.current = 0;

	// Nothing to run: the block configuration is empty. The machine has to start in the inert
	// `canceled` state, because every other state stamps a marker from `plan[…]` on entry and
	// would dereference a trial that does not exist.
	if (plan.length === 0) {
		const empty = new FiniteStateMachine<MyStates, MyEvents>("canceled", {
			instruction: {}, baseline: {}, stimulus: {}, go: {}, rating: {}, confirm: {},
			pause: {}, canceled: {},
		});
		cancel_callback();
		return empty;
	}

	let log: Log = {
		baseline_time: "",
		stimulus_time: "",
		go_time: "",
		rating_time: ""
	}

	// Advances the global trial counter and returns the next state, or ends the experiment.
	// A pause block renders its own screen; every other entry starts a fresh trial at the
	// baseline. No logging happens here, so pauses never write a trial row.
	function next_state(): MyStates | void {
		ExperimentIteration.current += 1;
		if (ExperimentIteration.current < plan.length) {
			return plan[ExperimentIteration.current].kind === "pause" ? "pause" : "baseline";
		} else {
			// fertig save data
			cancel_callback();
		}
	}

	// Logs the finished trial and moves on. Shared by the rating screen and by the
	// confirmation screen that replaces it on trials without a stimulus. The block bookkeeping
	// is stamped here so the screens never need to know about blocks.
	function advance(data: any): MyStates | void {
		const trial = plan[ExperimentIteration.current];
		const duration = durations[ExperimentIteration.current];
		let x = {
			...log,
			...data,
			baseline_speed: SpeedState.current,
			modification: duration.kind,
			effective_speed:
				((LengthState.current as number) /
					(duration.time / 1000)) *
				3.6,
			name: images.current?.name,
			n_valence: images.current?.valence,
			n_arousal: images.current?.arousal,
			block: trial.block + 1,
			trial_in_block: trial.trial_in_block + 1,
			block_type: trial.kind,
		};
		console.log(x);
		invoke("add_rating", { rating: x });

		return next_state();
	}

	// A pause carries nothing but its position in the plan. Logging it keeps the CSV a complete
	// record of the run, the columns a trial fills are left empty.
	function log_pause(trial: PlannedTrial): void {
		invoke("add_rating", {
			rating: {
				block: trial.block + 1,
				trial_in_block: trial.trial_in_block + 1,
				block_type: trial.kind,
				modification: "none",
			},
		});
	}

	// A pause block can be the very first entry, in which case the run opens on the break.
	const first_trial_state: MyStates = plan[0].kind === "pause" ? "pause" : "baseline";
	// A test run has no instructions, it is started from inside them.
	const initial: MyStates = mode === "experiment" ? "instruction" : first_trial_state;

	const experiment_state_machine = new FiniteStateMachine<MyStates, MyEvents>(
		initial,
		{
			// The instructions are their own phase, not part of the baseline. They run for as
			// long as the experimenter needs, so entering the baseline here would stamp its
			// marker minutes before the fixation cross actually appears.
			instruction: {
				_enter: async () => {
					await publish_event(sessionMarker());
					// Warm the first trial's image so the baseline marker can carry its id
					// without waiting for the fetch.
					images.prefetch(0);
				},
				// A test run opened from the instructions drives the same global trial counter
				// and draws from the same image pool. Put both back as soon as it closes, which
				// leaves the prefetch time to settle before the real run starts.
				test_finished: () => {
					ExperimentIteration.current = 0;
					images.reset();
				},
				instructed: () => {
					return first_trial_state;
				},
				cancel: () => {
					cancel_callback();
					return "canceled";
				},
			},
			baseline: {
				_enter: async () => {
					log.baseline_time = new Date().toISOString();
					const iteration = ExperimentIteration.current;
					// Resolves right away, the image was prefetched during the previous trial.
					// Only then does `images.current` belong to this trial.
					await images.promote(iteration);
					await publish_event(eventFromTrial(plan[iteration], "baseline", {
						image_id: images.current?.id,
						speed: durations[iteration].kind,
						type: marker_type,
					}));
					images.prefetch(iteration + 1);
				},
				start: () => {
					return "stimulus";
				},
				cancel: () => {
					cancel_callback();
					return "canceled";
				},
			},
			stimulus: {
				_enter: async () => {
					log.stimulus_time = new Date().toISOString();
					const trial = plan[ExperimentIteration.current];
					const duration = durations[ExperimentIteration.current];
					await publish_event(eventFromTrial(trial, "stimulus", {
						image_id: images.current?.id,
						speed: duration.kind,
						type: marker_type,
					}));
					const random = jittered_duration(StimulusModState.current);
					stimulus_debounce(experiment_state_machine, random * 1000)
				},
				s_fin: "go",
				cancel: () => {
					cancel_callback();
					return "canceled";
				},
			},
			go: {
				_enter: async () => {
					log.go_time = new Date().toISOString();
					const trial = plan[ExperimentIteration.current];
					const duration = durations[ExperimentIteration.current];
					await publish_event(eventFromTrial(trial, "go", {
						image_id: images.current?.id,
						speed: duration.kind,
						type: marker_type,
					}));
				},
				// Without a stimulus there is nothing to rate, the subject only confirms.
				g_fin: () => {
					return plan[ExperimentIteration.current]?.kind === "stimulus" &&
						(Settings.current.rating.arousal || Settings.current.rating.valence) ? "rating" : "confirm";
				},
				cancel: () => {
					cancel_callback();
					return "canceled";
				},
				_exit: () => {
				},
			},
			rating: {
				_enter: async () => {
					log.rating_time = new Date().toISOString();
					const trial = plan[ExperimentIteration.current];
					const duration = durations[ExperimentIteration.current];
					await publish_event(eventFromTrial(trial, "rating_prompt", {
						image_id: images.current?.id,
						speed: duration.kind,
						type: marker_type,
					}));
				},
				rated: (data: any) => {
					return advance(data);
				},
				cancel: () => {
					cancel_callback();
				},
			},
			// Stands in for `rating` on trials without a stimulus. It reuses the `Rating`
			// marker, the response phase is the same, only the CSV `has_stimulus` column
			// tells the two apart.
			confirm: {
				_enter: async () => {
					log.rating_time = new Date().toISOString();
					const trial = plan[ExperimentIteration.current];
					const duration = durations[ExperimentIteration.current];
					await publish_event(eventFromTrial(trial, "rating_prompt", {
						image_id: images.current?.id,
						speed: duration.kind,
						type: marker_type,
					}));
				},
				confirmed: (data: any) => {
					return advance(data);
				},
				cancel: () => {
					cancel_callback();
				},
			},
			// A break between blocks. It carries no trial, so it neither logs nor takes an
			// image out of the pool; it only waits for the experimenter to continue. Reuses the
			// Idle marker until the marker system is reworked.
			pause: {
				_enter: async () => {
					const iteration = ExperimentIteration.current;
					// A pause shows no image; promoting clears the one from the trial before it.
					await images.promote(iteration);
					await publish_event(pauseMarker(plan[iteration]));
					images.prefetch(iteration + 1);
				},
				continued: () => {
					log_pause(plan[ExperimentIteration.current]);
					return next_state();
				},
				cancel: () => {
					cancel_callback();
					return "canceled";
				},
			},
			// This is used to pseudo cancel the debounce since those are now created on the fly 
			// there is no trivial way of canceling them. To work around this we now have the caneceled
			// state. This state is only ever used when the experiment has been canceled and intentionally
			// has no event handlers. This leads to all ongoing debounces to fire without effect.
			canceled: {}
		},
	);
	return experiment_state_machine
}

