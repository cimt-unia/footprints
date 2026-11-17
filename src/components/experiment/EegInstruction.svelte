<script lang="ts">
	import type { ExperimentStateProps } from "./types.js";
	import { _ } from "svelte-i18n";

	// Tells the subject what the coming EEG recording asks of them. The recording starts on
	// Enter, so the experimenter can wait until the subject has settled.
	const props: ExperimentStateProps = $props();
	const kind = props.current_trial.block.kind;

	function onKeyDown(e: KeyboardEvent) {
		if (e.key !== "Enter") {
			return;
		}
		props.state_machine.send("confirmed");
	}
</script>

<div class="container m-auto flex h-screen flex-col items-center justify-center gap-6 text-center">
	<h2 class="h2">{$_(`eeg.${kind}.instruction`)}</h2>
	<p class="opacity-75">{$_("confirm.continue")}</p>
</div>

<svelte:window on:keydown|preventDefault={onKeyDown} />
