<script lang="ts">
	import FixationCross from "./FixationCross.svelte";
	import type { ExperimentStateProps } from "./types.js";

	// Unused, but the runners hand every screen the same props.
	const _props: ExperimentStateProps = $props();
</script>

<div class="fixation-cross-container">
	<FixationCross />
</div>

<style>
	/* The same layout as the baseline, see `Baseline.svelte`. */
	.fixation-cross-container {
		background-color: white;
		display: flex;
		justify-content: center;
		align-items: center;
		width: 80%;
		height: 80vh;
		position: relative;
		margin-top: 8vh;
		left: 10%;
	}
</style>
