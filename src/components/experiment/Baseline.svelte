<script lang="ts">
	import Countdown from "$components/Countdown.svelte";
	import { baseline_debounce } from "$lib/debounce.js";
	import { invoke } from "@tauri-apps/api/core";
	import type { ExperimentStateProps } from "./types.js";
	import { Settings } from "$lib/settings_state.js";
	import { BaselineModState } from "$lib/baseline_time_mod.js";
	import Instruction from "./Instruction.svelte";

	let { running = $bindable(), state_machine }: ExperimentStateProps =
		$props();
	const max =
		(BaselineModState.current.duration as number) +
		(BaselineModState.current.jitter as number);
	const min =
		(BaselineModState.current.duration as number) -
		(BaselineModState.current.jitter as number);
	let random = Math.random() * (max - min + 1) + min;
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
		<div
			class="fixation-cross"
			style="--cross-size: 20px; --cross-thickness: 2px; --cross-color: black;"
		></div>
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

	.fixation-cross {
		width: var(--cross-size);
		height: var(--cross-size);
		position: relative;
	}

	.fixation-cross::before,
	.fixation-cross::after {
		content: "";
		position: absolute;
		background-color: var(--cross-color);
	}

	/* Horizontal line */
	.fixation-cross::before {
		left: 50%;
		top: 50%;
		width: var(--cross-size);
		height: var(--cross-thickness);
		transform: translate(-50%, -50%);
	}

	/* Vertical line */
	.fixation-cross::after {
		left: 50%;
		top: 50%;
		width: var(--cross-thickness);
		height: var(--cross-size);
		transform: translate(-50%, -50%);
	}
</style>
