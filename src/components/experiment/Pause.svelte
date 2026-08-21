<script lang="ts">
	import type { ExperimentStateProps } from "./types.js";
	import { _ } from "svelte-i18n";

	// A break between blocks. There is no image, no walking and nothing to rate; the screen
	// just waits for the experimenter to press Enter before the next block starts.
	const props: ExperimentStateProps = $props();

	function onKeyDown(e: KeyboardEvent) {
		if (e.key !== "Enter") {
			return;
		}
		props.state_machine.send("continued");
	}
</script>

<div class="container flex m-auto items-center justify-center h-screen">
	<h2 class="h2">{$_("pause.continue")}</h2>
</div>

<svelte:window on:keydown|preventDefault={onKeyDown} />
