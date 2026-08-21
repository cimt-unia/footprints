<script lang="ts">
	import type { ExperimentStateProps } from "./types.js";
	import { SpeedState } from "$lib/speed_state.js";
	import { LengthState } from "$lib/length_state.js";
	import { _ } from "svelte-i18n";

	// Stands in for the rating screen on trials without a stimulus: there is no image to
	// rate, but the trial still waits for the subject before the next one starts.
	const props: ExperimentStateProps = $props();

	function onKeyDown(e: KeyboardEvent) {
		if (e.key !== "Enter") {
			return;
		}
		props.state_machine.send("confirmed", {
			baseline_speed: SpeedState.current,
			modification: props.duration.name,
			effective_speed:
				((LengthState.current as number) /
					(props.duration.time / 1000)) *
				3.6,
		});
	}
</script>

<div class="container flex m-auto items-center justify-center h-screen">
	<h2 class="h2">{$_("confirm.continue")}</h2>
</div>

<svelte:window on:keydown|preventDefault={onKeyDown} />
