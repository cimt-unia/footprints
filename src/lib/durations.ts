import type { PlannedTrial } from "./blocks_state.js";

/** A trial's walking condition: the label written to the log and the time budget in ms. */
export interface Duration {
	name: string;
	time: number;
}

interface SpeedSettings {
	very_slow: boolean;
	slow: boolean;
	fast: boolean;
	very_fast: boolean;
}

/**
 * The enabled walking conditions. `Normal` is the time it takes to walk `length` meters at
 * `speed` km/h, the enabled variants scale that time.
 */
export function speed_conditions(
	length: number,
	speed: number,
	settings: SpeedSettings
): Duration[] {
	const normal = (Number(length) / (Number(speed) / 3.6)) * 1000;
	const conditions: Duration[] = [{ name: "Normal", time: normal }];
	const variants: [boolean, string, number][] = [
		[settings.very_slow, "Very slow", 1.2],
		[settings.slow, "Slow", 1.1],
		[settings.fast, "Fast", 0.9],
		[settings.very_fast, "Very fast", 0.8],
	];
	for (const [enabled, name, factor] of variants) {
		if (enabled) {
			conditions.push({ name, time: normal * factor });
		}
	}
	return conditions;
}

function shuffle<T>(arr: T[]): T[] {
	for (let i = arr.length - 1; i > 0; i--) {
		const j = Math.floor(Math.random() * (i + 1));
		const temp = arr[i];
		arr[i] = arr[j];
		arr[j] = temp;
	}
	return arr;
}

/**
 * One duration per planned trial, balanced within each block.
 *
 * A block of `n` trials over `k` conditions gets `floor(n / k)` complete copies of the
 * condition list, so as long as `k` divides `n` every condition appears equally often inside
 * every block. The `n % k` remainder trials are drawn from a cycle that carries over between
 * blocks and is only reshuffled once exhausted, so the leftovers do not always fall on the
 * same conditions and the run stays balanced overall as well. Shuffling happens per block, so
 * a condition never leaks into a neighbouring block.
 *
 * The result has exactly `plan.length` entries, so it is index aligned with the trial plan.
 */
export function balanced_durations(
	conditions: Duration[],
	plan: PlannedTrial[]
): Duration[] {
	if (conditions.length === 0) {
		return [];
	}

	const durations: Duration[] = [];
	let carry: Duration[] = [];

	let start = 0;
	while (start < plan.length) {
		// A pause occupies a plan slot but carries no walking; give it a placeholder so the
		// result stays index aligned without drawing from or perturbing the balanced carry.
		if (plan[start].kind === "pause") {
			durations.push({ name: "Pause", time: 0 });
			start += 1;
			continue;
		}

		// The plan lists a block's trials consecutively.
		let end = start;
		while (end < plan.length && plan[end].block === plan[start].block) {
			end++;
		}
		const trials = end - start;

		const block: Duration[] = [];
		const complete = Math.floor(trials / conditions.length);
		for (let copy = 0; copy < complete; copy++) {
			block.push(...conditions);
		}
		while (block.length < trials) {
			if (carry.length === 0) {
				carry = shuffle([...conditions]);
			}
			block.push(carry.pop()!);
		}

		durations.push(...shuffle(block));
		start = end;
	}

	return durations;
}
