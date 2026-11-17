import { invoke } from "@tauri-apps/api/core";
import { eeg_duration, type EegBlock, type EegKind, type PlannedTrial } from "./blocks_state.js";
import type { SpeedKind } from "./durations.js";
import type { InstructionSet } from "./instructions.js";

/**
 * Which kind of run a trial marker belongs to. A test run draws from the same plan as a real
 * one, the tag is what keeps its markers out of the recorded data.
 */
export type MarkerRunKind = "stimulus" | "test";

/** The phases every trial passes through, whether or not it shows an image. */
export type TrialPhase = "baseline" | "stimulus" | "go" | "rating_prompt";

/** The moment inside a stimulus trial. Only the rating phases carry data. */
export type StimulusEvent =
	| { state: TrialPhase }
	| { state: "rating_valence", rating: number }
	| { state: "rating_arousal", rating: number };

/**
 * The moment of a resting state EEG recording. `start` carries how long the recording is set to
 * run, `end` is stamped once that time is over.
 */
export type EegEvent =
	| { state: "instruction" | "end" }
	| { state: "start", duration_s: number };

/** The moment inside a calibration step. Only the result carries the calibrated speed. */
export type CalibrationEvent =
	| { state: "start" | "stop" | "discarded" | "confirmed" }
	| { state: "result", speed_kmh: number };

/** The shared fields of a trial marker. */
interface TrialMarker {
	/** 1 based index of the block inside the plan. */
	block: number,
	/** 1 based index of the trial inside its block. */
	trial: number,
	speed: SpeedKind
}

/**
 * One App event as the backend expects it, tagged on the block that emitted it: that is what
 * decides which fields a marker has. The Rust side mirrors this union in `lsl.rs`, where the
 * packed `image_id` is unpacked into its quadrant before the marker goes out on the stream.
 */
export type LsLMarker =
	| { type: "session", instructions?: InstructionsShown }
	| { type: "pause", block: number }
	| { type: "calibration", step: number } & CalibrationEvent
	| { type: "neutral" } & TrialMarker & { state: TrialPhase }
	| { type: EegKind, block: number } & EegEvent
	| { type: MarkerRunKind | "static_stimulus", image_id?: number } & TrialMarker & StimulusEvent;

/** Which instruction pages a session showed, see `InstructionsShown` in `lsl.rs`. */
export interface InstructionsShown {
	lang: string;
	/** The file names, in the order they were shown. */
	pages: string[];
	hash: string;
}

/** Everything about a trial marker that the trial itself does not determine. */
export interface MarkerDetails {
	/** Stimulus image of the trial, left out on trials that show none. */
	image_id?: number,
	/** Walking condition of the trial, defaults to `none` for the slots without walking. */
	speed?: SpeedKind,
	/**
	 * Overrides the trial's own kind, which a test run needs: its plan holds ordinary stimulus
	 * trials, only the recording must not confuse them with the real thing.
	 */
	type?: MarkerRunKind
}

/** The tag of a trial that shows an image, which only a run override can change. */
function image_trial_type(trial: PlannedTrial, details: MarkerDetails): MarkerRunKind | "static_stimulus" {
	if (details.type) {
		return details.type;
	}
	return trial.block.kind === "static_stimulus" ? "static_stimulus" : "stimulus";
}

/** A marker for one trial of the plan, for the phases that carry no data. */
export function eventFromTrial(trial: PlannedTrial, phase: TrialPhase, details: MarkerDetails = {}): LsLMarker {
	if (trial.block.kind === "pause") {
		return pauseMarker(trial);
	}
	if (trial.block.kind === "eyes_open" || trial.block.kind === "eyes_closed") {
		// It has no trial phases, see `eegMarker`.
		throw new Error("An EEG block publishes `eegMarker`, not trial markers");
	}

	const common = {
		block: trial.block_nr + 1,
		trial: trial.trial_in_block + 1,
		speed: details.speed ?? "none",
		state: phase
	} satisfies TrialMarker & { state: TrialPhase };

	// A neutral trial shows a fixation cross, it has no image and is never rated.
	if (trial.block.kind === "neutral") {
		return { type: "neutral", ...common };
	}

	return {
		type: image_trial_type(trial, details),
		image_id: details.image_id,
		...common
	}
}

/**
 * A rating of the image of a stimulus trial. Only a stimulus trial is rated, a trial without an
 * image is confirmed instead and stays on `rating_prompt`.
 */
export function ratingMarker(
	trial: PlannedTrial,
	rating: Extract<StimulusEvent, { rating: number }>,
	details: MarkerDetails = {}
): LsLMarker {
	return {
		type: image_trial_type(trial, details),
		block: trial.block_nr + 1,
		trial: trial.trial_in_block + 1,
		speed: details.speed ?? "none",
		image_id: details.image_id,
		...rating
	}
}

/** A break between blocks. It has no phases, no walking and nothing to rate. */
export function pauseMarker(trial: PlannedTrial): LsLMarker {
	return { type: "pause", block: trial.block_nr + 1 }
}

/**
 * A moment of a resting state EEG recording. The block is a single recording, so it has no
 * trial, no walking and nothing to rate.
 */
export function eegMarker(trial: PlannedTrial & { block: EegBlock }, state: EegEvent["state"]): LsLMarker {
	const marker = { type: trial.block.kind, block: trial.block_nr + 1 };
	if (state === "start") {
		return { ...marker, state, duration_s: eeg_duration(trial.block) };
	}
	return { ...marker, state };
}

/**
 * A marker of the speed calibration, which has no block and no trial plan of its own: the step
 * is its own counter and the state alone says what happened to it.
 */
export function calibrationMarker(step: number, event: CalibrationEvent): LsLMarker {
	return { type: "calibration", step, ...event }
}

/**
 * Marks the start of a session. It belongs to no block and no trial, but records the
 * instruction pages the subject is about to read.
 */
export function sessionMarker(instructions?: InstructionSet): LsLMarker {
	if (!instructions) {
		return { type: "session" }
	}
	return {
		type: "session",
		instructions: {
			lang: instructions.lang,
			pages: instructions.pages.map((page) => page.file),
			hash: instructions.hash,
		},
	}
}

export async function publish_event(event: LsLMarker) {
	await invoke("publish_lsl", { event });
}

/**
 * Publishes without blocking the caller, for the synchronous event handlers of the calibration.
 * Those drive a timer and must not wait on an IPC round trip.
 */
export function publish_event_detached(event: LsLMarker): void {
	publish_event(event).catch((e) => console.error("Failed to publish an LsL marker", e));
}
