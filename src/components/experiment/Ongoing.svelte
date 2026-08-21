<script lang="ts">
	import Countdown from "$components/Countdown.svelte";
	import Sprite from "./Sprite.svelte";
	import FixationCross from "./FixationCross.svelte";
	import type { ExperimentStateProps } from "./types.js";
	import { invoke } from "@tauri-apps/api/core";
	import { Settings } from "$lib/settings_state.js";

	const { state_machine, duration, img_url }: ExperimentStateProps = $props();

	let start_go = $derived(state_machine.current !== "stimulus");

	$effect(() => {
		if (start_go && Settings.current.sound_cue) {
			invoke("play_sound");
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
			<Sprite {duration} {state_machine} {w} y={-55} />
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
