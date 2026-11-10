import { invoke } from "@tauri-apps/api/core";
import { shows_image, type PlannedTrial } from "./blocks_state.js";

/** The image as it comes back from the backend, with the webp bytes still raw. */
interface RawImage {
	name: string;
	valence: "Low" | "High";
	arousal: "Low" | "High";
	data: number[];
}

export interface TrialImage {
	/** Backend identifier, quadrant in the upper 2 bits. Also goes into the LsL marker. */
	id: number;
	name: string;
	valence: "Low" | "High";
	arousal: "Low" | "High";
	url: string;
}

/**
 * Owns the stimulus image of the running trial.
 *
 * The image of a trial is fetched while the *previous* trial is still on screen, so that it is
 * already in hand the moment the state machine enters the next one. The LsL marker stamped on
 * entry carries the image id, and a fetch started at that point only resolves after the marker
 * has already gone out, which would stamp the previous trial's id (or none at all on the very
 * first trial). Prefetching keeps the id correct without delaying the marker.
 */
export class TrialImages {
	#plan: PlannedTrial[];
	#current = $state<TrialImage | undefined>(undefined);
	#pending: Promise<TrialImage | undefined> | undefined;
	#pending_for = -1;
	/** The pool reset, every fetch waits for it so the first trial cannot outrun it. */
	#ready: Promise<unknown>;

	constructor(plan: PlannedTrial[]) {
		this.#plan = plan;
		this.#ready = invoke("reset_images");
	}

	/** The image of the trial on screen, `undefined` on trials that show no stimulus. */
	get current(): TrialImage | undefined {
		return this.#current;
	}

	/** Starts loading the image of `iteration` in the background. */
	prefetch(iteration: number): void {
		if (this.#pending_for === iteration) {
			return;
		}
		this.#pending_for = iteration;
		const pending = this.#load(iteration);
		this.#pending = pending;
		// Nothing awaits a prefetch, so a failure has to be absorbed here to avoid an
		// unhandled rejection. `promote` reports it when it picks the result up.
		pending.catch(() => undefined);
	}

	/**
	 * Makes the image of `iteration` the current one. Normally resolves within a microtask
	 * because the image was already prefetched; it only really waits when the prefetch was
	 * skipped, which is the case for the first trial of a run.
	 */
	async promote(iteration: number): Promise<void> {
		this.prefetch(iteration);
		const pending = this.#pending;
		let next: TrialImage | undefined;
		try {
			next = await pending;
		} catch (e) {
			console.error("Failed to load the stimulus image", e);
		}
		// A newer prefetch was promoted while this one was in flight, drop the stale result.
		if (pending !== this.#pending) {
			revoke(next);
			return;
		}
		revoke(this.#current);
		this.#current = next;
	}

	/**
	 * Refills the backend pool and re-warms the first trial, discarding whatever is held.
	 * Used after a test run, which draws from the same pool as the experiment around it.
	 */
	reset(): void {
		this.release();
		this.#ready = invoke("reset_images");
		this.prefetch(0);
	}

	/** Drops the image on screen and any prefetch still in flight. */
	release(): void {
		revoke(this.#current);
		this.#current = undefined;
		const pending = this.#pending;
		this.#pending = undefined;
		this.#pending_for = -1;
		pending?.then(revoke, () => undefined);
	}

	async #load(iteration: number): Promise<TrialImage | undefined> {
		// Only stimulus and static stimulus trials show an image; neutral trials (fixation cross)
		// and pauses must not take an image out of the pool.
		const trial = this.#plan[iteration];
		if (!trial || !shows_image(trial.kind)) {
			return undefined;
		}
		await this.#ready;
		const [id, img] = await invoke<[number, RawImage]>("get_image");
		const blob = new Blob([new Uint8Array(img.data).buffer], { type: "image/webp" });
		return {
			id,
			name: img.name,
			valence: img.valence,
			arousal: img.arousal,
			url: URL.createObjectURL(blob),
		};
	}
}

function revoke(img: TrialImage | undefined): void {
	if (img?.url) {
		URL.revokeObjectURL(img.url);
	}
}
