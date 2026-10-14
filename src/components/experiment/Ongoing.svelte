<script lang="ts">
	import Countdown from "$components/Countdown.svelte";
	import Sprite from "./Sprite.svelte";
	import FixationCross from "./FixationCross.svelte";
	import type { ExperimentStateProps } from "./types.js";
	import { go_cue } from "$lib/go_cue.js";

	const { state_machine, duration, img_url, cue_target }: ExperimentStateProps =
		$props();

	let start_go = $derived(state_machine.current !== "stimulus");

	$effect(() => {
		if (start_go) {
			go_cue(cue_target);
		}
	});
	let w: number = $state(0);
</script>

<div class="fixation-cross-container" class:no-stimulus={!img_url}>
	{#if img_url}
		<img class="h-full w-full object-contain" src={img_url} alt="stimulus" />
	{:else}
		<!-- Trials of a block without a stimulus show the same cross as the baseline. -->
		<FixationCross />
	{/if}
</div>
<div>
	{#if start_go}
		<div style="w-screen" bind:clientWidth={w}>
			<Sprite {duration} {state_machine} {w} y={0} {cue_target} />
		</div>
	{:else}
		<div class="flex container m-auto justify-center">
			<Countdown duration={3} />
		</div>
	{/if}
</div>

<style>
	.fixation-cross-container {
		display: flex;
		justify-content: center;
		align-items: center;
		width: 80%;
		height: 80vh;
		position: relative;
		top: 5%;
		left: 10%;
	}

	.no-stimulus {
		background-color: white;
	}
</style>
