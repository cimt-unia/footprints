import type { PlannedTrial, SpeedVariations } from "./blocks_state.js";

/**
 * A trial's walking condition, as a stable discriminator. Ordered slowest to fastest in the LsL
 * marker, `none` is for plan slots that involve no walking at all.
 */
export type SpeedKind = "none" | "very_slow" | "slow" | "normal" | "fast" | "very_fast";

/**
 * A trial's walking condition: the discriminator that goes into both the LsL marker and the log,
 * and the time budget in ms.
 */
export interface Duration {
	kind: SpeedKind;
	time: number;
}

/**
 * The enabled walking conditions. `Normal` is the time it takes to walk `length` meters at
 * `speed` km/h, the enabled variants scale that time.
 */
export function speed_conditions(
	length: number,
	speed: number,
	settings: SpeedVariations
): Duration[] {
	const normal = (Number(length) / (Number(speed) / 3.6)) * 1000;
	const conditions: Duration[] = [{ kind: "normal", time: normal }];
	const variants: [boolean, SpeedKind, number][] = [
		[settings.very_slow, "very_slow", 1.2],
		[settings.slow, "slow", 1.1],
		[settings.fast, "fast", 0.9],
		[settings.very_fast, "very_fast", 0.8],
	];
	for (const [enabled, kind, factor] of variants) {
		if (enabled) {
			conditions.push({ kind, time: normal * factor });
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
 * A block of `n` trials over its `k` conditions gets `floor(n / k)` complete copies of the
 * condition list, so as long as `k` divides `n` every condition appears equally often inside
 * every block. The `n % k` remainder trials are drawn from a cycle that carries over between
 * blocks and is only reshuffled once exhausted, so the leftovers do not always fall on the
 * same conditions and the run stays balanced overall as well. Blocks only share a cycle when
 * they use the same conditions, a leftover could otherwise put a condition into a block that
 * has it turned off. Shuffling happens per block, so a condition never leaks into a neighbouring
 * block.
 *
 * The result has exactly `plan.length` entries, so it is index aligned with the trial plan.
 */
export function balanced_durations(
	length: number,
	speed: number,
	plan: PlannedTrial[]
): Duration[] {
	const durations: Duration[] = [];
	const carries = new Map<string, Duration[]>();

	let start = 0;
	while (start < plan.length) {
		// A pause or a static trial occupies a plan slot but carries no walking; give it a
		// placeholder so the result stays index aligned without drawing from or perturbing the
		// balanced carry.
		if (plan[start].kind === "pause" || plan[start].kind === "static_stimulus") {
			durations.push({ kind: "none", time: 0 });
			start += 1;
			continue;
		}

		// The plan lists a block's trials consecutively.
		let end = start;
		while (end < plan.length && plan[end].block === plan[start].block) {
			end++;
		}
		const trials = end - start;

		const conditions = speed_conditions(length, speed, plan[start].settings.speeds);
		const cycle = conditions.map((condition) => condition.kind).join();
		let carry = carries.get(cycle) ?? [];

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
		carries.set(cycle, carry);

		durations.push(...shuffle(block));
		start = end;
	}

	return durations;
}
