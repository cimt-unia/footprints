<script lang="ts">
	import Countdown from "$components/Countdown.svelte";
	import { baseline_debounce } from "$lib/debounce.js";
	import type { ExperimentStateProps } from "./types.js";
	import { jittered_duration } from "$lib/jitter.js";
	import FixationCross from "./FixationCross.svelte";

	// The baseline is only the fixation cross. The instructions that precede the very first one
	// are their own state, so that entering this screen lines up with the LsL marker.
	const { state_machine, current_trial }: ExperimentStateProps = $props();
	const random = jittered_duration(current_trial.settings.baseline);
	baseline_debounce(state_machine, random * 1000);
</script>

<div class="fixation-cross-container">
	<FixationCross />
</div>
<div class="flex mt-5 container m-auto justify-center">
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
</style>
