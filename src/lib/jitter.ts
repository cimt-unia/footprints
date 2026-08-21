// Shared by the baseline and stimulus phases: both draw a phase duration from
// `duration +- jitter`.

interface TimeMod {
	duration: number | "";
	jitter: number | "";
}

/**
 * Duration in seconds, drawn uniformly from [duration - jitter, duration + jitter].
 *
 * The fields are `number | ""` because clearing the input in the settings
 * persists an empty string, an empty field counts as 0 and the result is never
 * negative so a phase can not be skipped by a bad setting.
 */
export function jittered_duration(mod: TimeMod): number {
	const duration = Number(mod.duration) || 0;
	const jitter = Math.abs(Number(mod.jitter) || 0);

	return Math.max(0, duration - jitter + Math.random() * 2 * jitter);
}
