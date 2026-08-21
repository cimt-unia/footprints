<script lang="ts">
	import { SpeedState } from "$lib/speed_state.js";
	import { AppBar, Modal } from "@skeletonlabs/skeleton-svelte";
	import { CalibrationLengthState } from "$lib/calibration_length_state.js";
	import Calibration from "./Calibration.svelte";

	let openState = $state(false);

	let steps = $state(2);

	let speed = $state(null);

	function set_state() {
		SpeedState.current = speed ?? "";
	}
</script>

<AppBar>
	<h2 class="h2">Speed calibration</h2>
</AppBar>
<form
	class="mx-auto my-10 w-full max-w-md space-y-4 flex flex-col"
	onsubmit={() => (openState = true)}
>
	<label class="label">
		<span class="label-text">Calibration steps:</span>
		<input
			type="number"
			class="input"
			placeholder="Speed"
			required
			bind:value={steps}
		/>
	</label>
	<label class="label">
		<span class="label-text">Walked distance per step in m</span>
		<input
			type="number"
			class="input"
			placeholder="Length in m"
			required
			bind:value={CalibrationLengthState.current}
		/>
	</label>
	<button
		class="btn preset-filled-primary-500 dark:preset-filled-primary-500"
		type="submit">Start calibration</button
	>
	<Modal
		open={openState}
		onOpenChange={(e) => (openState = e.open)}
		contentBase="card bg-surface-100-900 space-y-4 shadow-xl min-w-screen min-h-screen"
		backdropClasses="backdrop-blur-sm"
	>
		{#snippet content()}
			<Calibration
				{steps}
				length={CalibrationLengthState.current as number}
				bind:speed
				bind:openState
			/>
		{/snippet}
	</Modal>
</form>
{#if speed}
	<form
		class="mx-auto my-10 w-full max-w-md space-y-4 flex flex-col"
		onsubmit={set_state}
	>
		<p>Calculated Speed: {speed}</p>
		<button onclick={set_state}>Set State</button>
	</form>
{/if}

<style>
	* {
		background-color: text-secondary-contrast-200-800;
	}
</style>
