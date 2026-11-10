import { PersistedState } from "runed";
import { eventFromTrial, pauseMarker, publish_event, sessionMarker, type TrialPhase } from "./lsl.js";
import { invoke } from "@tauri-apps/api/core";
import { stimulus_debounce } from "./debounce.js";
import { jittered_duration } from "./jitter.js";
import { shows_image, type BlockKind, type PlannedTrial } from "./blocks_state.js";
import type { Duration } from "./durations.js";
import { SpeedState } from "./speed_state.js";
import { LengthState } from "./length_state.js";
import type { TrialImages } from "./trial_images.svelte.js";
import type { InstructionSet } from "./instructions.js";

/** A screen a plan entry passes through. Which ones, and in which order, is up to its block kind. */
export type Phase = "baseline" | "stimulus" | "go" | "rating" | "confirm" | "pause";
export type MyStates = "instruction" | Phase | "finished" | "canceled";
export type MyEvents = "start" | "s_fin" | "g_fin" | "rated" | "confirmed" | "continued" | "cancel" | "instructed" | "test_finished";

// global reactive variable to track the experiment progress
export const ExperimentIteration = new PersistedState("ex_iter", 0);

export interface ExperimentMachine {
	/** The state the run is in. Reactive. */
	readonly current: MyStates;
	/**
	 * Identifies the screen to render, the runners key on it. It changes whenever a new plan
	 * entry begins, also between two entries that start in the same phase (two pauses in a row),
	 * but not from `stimulus` to `go`, which share the walking screen.
	 */
	readonly screen: string;
	/** Hands an event to the current state and returns the state the run is in afterwards. */
	send(event: MyEvents, data?: object): MyStates;
}

// A phase a trial did not pass through stays absent, the backend reads a missing key as an empty
// cell but rejects the whole row on an empty string.
interface Log {
	baseline_time?: string,
	stimulus_time?: string,
	go_time?: string,
	rating_time?: string,
}

interface PhaseSpec {
	/**
	 * The only event that ends this phase. Any other, e.g. from a timer that outlived the phase
	 * it was started in, is ignored.
	 */
	done: MyEvents;
	/**
	 * Stamps the marker and prepares the screen. Runs every time the phase is entered, also when
	 * the previous entry ended in the same phase.
	 */
	enter(trial: PlannedTrial, iteration: number): Promise<void>;
	/** Runs once `done` arrives, before the next phase is entered, with what the screen sent along. */
	finish?(trial: PlannedTrial, iteration: number, data?: object): void;
}

// The screen that closes a trial. Only a trial with an image has something to rate, and only
// when its block enables a rating, otherwise the subject just confirms.
function response_phase(trial: PlannedTrial): "rating" | "confirm" {
	const { kind, settings } = trial;
	return shows_image(kind) && (settings.rating.arousal || settings.rating.valence) ? "rating" : "confirm";
}

/**
 * The phases one plan entry runs through, in order. This is the one place that knows how block
 * kinds differ in their course, a new kind only needs its flow added here.
 */
const flows: Record<BlockKind, (trial: PlannedTrial) => Phase[]> = {
	stimulus: (trial) => ["baseline", "stimulus", "go", response_phase(trial)],
	// Walks like a stimulus trial, with a fixation cross in the image's place.
	neutral: (trial) => ["baseline", "stimulus", "go", response_phase(trial)],
	// No walking, the image leads straight to the response.
	static_stimulus: (trial) => ["baseline", "stimulus", response_phase(trial)],
	// A break between blocks, it only waits for the experimenter to continue.
	pause: () => ["pause"],
};

function is_phase(state: MyStates): state is Phase {
	return state !== "instruction" && state !== "finished" && state !== "canceled";
}

/**
 * Runs the plan entry by entry, each through the phases its block kind lays out in `flows`.
 * Entering a phase is always explicit, so every entry stamps its markers even when it starts in
 * the phase the previous one ended in.
 *
 * However the run ends, the last entry done or canceled, `cancel_callback` runs exactly once and
 * the machine settles in an inert state that ignores every further event. That lets the runners
 * send `cancel` unconditionally when they unmount, and lets late timers fire without effect.
 *
 * @param mode A test run skips the instructions and stamps every marker as `test`, so a
 * practice run cannot be mistaken for real data in the recording.
 * @param instructions The pages the instructions show, recorded in the session marker.
 */
export function create_state_machine(cancel_callback: () => void, plan: PlannedTrial[], durations: Duration[], images: TrialImages, mode: "experiment" | "test" = "experiment", instructions?: InstructionSet): ExperimentMachine {
	const marker_type = mode === "test" ? "test" as const : undefined;
	// We just created the experiment state machine so we are in the first iteration.
	ExperimentIteration.current = 0;

	let state: MyStates = $state("instruction");
	// The plan entry the run is on. Kept here rather than read from `ExperimentIteration`: a test
	// run opened from the instructions moves that shared counter, and must neither shift this run
	// nor remount the instructions behind it.
	let entry = $state(0);
	// The phases of the current entry and the position in them.
	let flow: Phase[] = [];
	let step = 0;
	let log: Log = {};

	function publish_trial_marker(trial: PlannedTrial, iteration: number, phase: TrialPhase) {
		return publish_event(eventFromTrial(trial, phase, {
			image_id: images.current?.id,
			speed: durations[iteration].kind,
			type: marker_type,
		}));
	}

	// The response screens look alike to the recording. `confirm` reuses the `Rating` marker,
	// only the CSV `has_stimulus` column tells the two apart.
	async function enter_response(trial: PlannedTrial, iteration: number) {
		log.rating_time = new Date().toISOString();
		await publish_trial_marker(trial, iteration, "rating_prompt");
	}

	// Logs the finished trial. The block bookkeeping is stamped here so the screens never need
	// to know about blocks.
	function log_trial(trial: PlannedTrial, iteration: number, data?: object) {
		const duration = durations[iteration];
		let x = {
			...log,
			...data,
			baseline_speed: SpeedState.current,
			modification: duration.kind,
			// A trial without walking has no time budget and so no speed.
			effective_speed: duration.time > 0
				? ((LengthState.current as number) /
					(duration.time / 1000)) *
				3.6
				: undefined,
			name: images.current?.name,
			n_valence: images.current?.valence,
			n_arousal: images.current?.arousal,
			block: trial.block + 1,
			trial_in_block: trial.trial_in_block + 1,
			block_type: trial.kind,
		};
		console.log(x);
		invoke("add_rating", { rating: x });
	}

	// A pause carries nothing but its position in the plan. Logging it keeps the CSV a complete
	// record of the run, the columns a trial fills are left empty.
	function log_pause(trial: PlannedTrial) {
		invoke("add_rating", {
			rating: {
				block: trial.block + 1,
				trial_in_block: trial.trial_in_block + 1,
				block_type: trial.kind,
				modification: "none",
			},
		});
	}

	const phases: Record<Phase, PhaseSpec> = {
		// The baseline is the fixation cross. Its screen ends it with `start` once its jittered
		// duration is over.
		baseline: {
			done: "start",
			enter: async (trial, iteration) => {
				// Starts from scratch so a trial that skips a phase cannot inherit that
				// phase's time from the trial before it.
				log = { baseline_time: new Date().toISOString() };
				// Resolves right away, the image was prefetched during the previous entry.
				// Only then does `images.current` belong to this trial.
				await images.promote(iteration);
				await publish_trial_marker(trial, iteration, "baseline");
				images.prefetch(iteration + 1);
			},
		},
		stimulus: {
			done: "s_fin",
			enter: async (trial, iteration) => {
				log.stimulus_time = new Date().toISOString();
				await publish_trial_marker(trial, iteration, "stimulus");
				const random = jittered_duration(trial.settings.stimulus);
				stimulus_debounce(machine, random * 1000);
			},
		},
		go: {
			done: "g_fin",
			enter: async (trial, iteration) => {
				log.go_time = new Date().toISOString();
				await publish_trial_marker(trial, iteration, "go");
			},
		},
		rating: { done: "rated", enter: enter_response, finish: log_trial },
		// Stands in for `rating` on trials without anything to rate.
		confirm: { done: "confirmed", enter: enter_response, finish: log_trial },
		// It carries no trial, so it takes no image out of the pool. Reuses the Idle marker until
		// the marker system is reworked.
		pause: {
			done: "continued",
			enter: async (trial, iteration) => {
				// A pause shows no image; promoting clears the one from the trial before it.
				await images.promote(iteration);
				await publish_event(pauseMarker(trial));
				images.prefetch(iteration + 1);
			},
			finish: log_pause,
		},
	};

	function enter_phase(phase: Phase) {
		state = phase;
		void phases[phase].enter(plan[entry], entry);
	}

	// Starts plan entry `index` at the first phase of its flow, or ends the run after the last.
	function begin_entry(index: number) {
		if (index >= plan.length) {
			// fertig save data
			end("finished");
			return;
		}
		entry = index;
		ExperimentIteration.current = index;
		flow = flows[plan[index].kind](plan[index]);
		step = 0;
		enter_phase(flow[0]);
	}

	// The state is set first: the callback closes the runner, whose teardown sends `cancel`,
	// which then finds the machine inert.
	function end(final: "finished" | "canceled") {
		state = final;
		cancel_callback();
	}

	// The instructions are their own phase, not part of the baseline. They run for as long as
	// the experimenter needs, so entering the baseline here would stamp its marker minutes
	// before the fixation cross actually appears.
	async function enter_instruction() {
		await publish_event(sessionMarker(instructions));
		// Warm the first trial's image so the baseline marker can carry its id without waiting
		// for the fetch.
		images.prefetch(0);
	}

	function send(event: MyEvents, data?: object): MyStates {
		if (state === "finished" || state === "canceled") {
			// Late timers and the runners' teardown land here on purpose, see above.
			return state;
		}
		if (event === "cancel") {
			end("canceled");
		} else if (state === "instruction") {
			if (event === "instructed") {
				begin_entry(0);
			} else if (event === "test_finished") {
				// A test run opened from the instructions draws from the same image pool and
				// moves the shared trial counter. Put both back as soon as it closes, which
				// leaves the prefetch time to settle before the real run starts.
				ExperimentIteration.current = 0;
				images.reset();
			} else {
				console.warn("No action defined for event", event, "in state", state);
			}
		} else if (is_phase(state) && event === phases[state].done) {
			phases[state].finish?.(plan[entry], entry, data);
			step += 1;
			if (step < flow.length) {
				enter_phase(flow[step]);
			} else {
				begin_entry(entry + 1);
			}
		} else {
			console.warn("No action defined for event", event, "in state", state);
		}
		return state;
	}

	const machine: ExperimentMachine = {
		get current() {
			return state;
		},
		get screen() {
			return `${entry}:${state === "go" ? "stimulus" : state}`;
		},
		send,
	};

	if (plan.length === 0) {
		// Nothing to run: the block configuration is empty.
		end("finished");
	} else if (mode === "experiment") {
		void enter_instruction();
	} else {
		// A test run has no instructions, it is started from inside them.
		begin_entry(0);
	}
	return machine;
}
