<script lang="ts">
	import Countdown from "$components/Countdown.svelte";
	import { baseline_debounce } from "$lib/debounce.js";
	import type { ExperimentStateProps } from "./types.js";
	import { jittered_duration } from "$lib/jitter.js";
	import { trial_settings } from "$lib/blocks_state.js";
	import FixationCross from "./FixationCross.svelte";

	// The baseline is only the fixation cross. The instructions that precede the very first one
	// are their own state, so that entering this screen lines up with the LsL marker.
	const { state_machine, current_trial }: ExperimentStateProps = $props();
	const random = jittered_duration(trial_settings(current_trial).baseline);
	baseline_debounce(state_machine, random * 1000);

	// A neutral trial's cross sits where the walking screen draws it, which leaves room below
	// for the speed indicator. Moving it when the walk starts reads to subjects like the go signal.
	const neutral = current_trial.block.kind === "neutral";
</script>

<div class="fixation-cross-container" class:neutral>
	<FixationCross />
</div>
<div class="flex container m-auto justify-center" class:mt-5={!neutral}>
	<Countdown duration={2} />
</div>

<style>
	.fixation-cross-container {
		background-color: white;
		display: flex;
		justify-content: center;
		align-items: center;
		width: 80%;
		height: 80vh; /* Or a specific height if you prefer */
		position: relative; /* Or relative, depending on your layout */
		margin-top: 8vh;
		left: 10%;
	}

	/* Exactly the walking screen's layout, see `Ongoing.svelte`. */
	.fixation-cross-container.neutral {
		margin-top: 0;
		top: 5%;
	}
</style>
