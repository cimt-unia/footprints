<script lang="ts">
	import Countdown from "$components/Countdown.svelte";
	import { baseline_debounce } from "$lib/debounce.js";
	import { invoke } from "@tauri-apps/api/core";
	import type { ExperimentStateProps } from "./types.js";
	import { Settings } from "$lib/settings_state.js";
	import { BaselineModState } from "$lib/baseline_time_mod.js";
	import { jittered_duration } from "$lib/jitter.js";
	import Instruction from "./Instruction.svelte";
	import FixationCross from "./FixationCross.svelte";

	let { running = $bindable(), state_machine }: ExperimentStateProps =
		$props();
	const random = jittered_duration(BaselineModState.current);
	async function start_experiment() {
		baseline_debounce(state_machine, random * 1000);
		running = true;
	}

	if (running) {
		baseline_debounce(state_machine, random * 1000);
	}
</script>

{#if running}
	<div class="fixation-cross-container">
		<FixationCross />
	</div>
	<div class="flex mt-5 container m-auto justify-center">
		<Countdown duration={2} />
	</div>
{:else}
	<Instruction cb={start_experiment} />
{/if}

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
