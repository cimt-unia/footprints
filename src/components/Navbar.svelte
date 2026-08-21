<script lang="ts">
	import { AppBar, Modal } from "@skeletonlabs/skeleton-svelte";
	import SettingsIcon from "@lucide/svelte/icons/settings";
	import Settings from "./Settings.svelte";

	let openState = $state(false);
</script>

<AppBar>
	{#snippet lead()}{/snippet}
	<h2 class="h2">Footprints experiment Setup</h2>
	{#snippet trail()}
		<Modal
			open={openState}
			onOpenChange={(e) => (openState = e.open)}
			triggerBase="btn preset-tonal"
			contentBase="bg-surface-100-900 p-4 space-y-4 shadow-xl w-[380px] h-screen"
			positionerJustify="justify-end"
			positionerAlign=""
			positionerPadding=""
			transitionsPositionerIn={{
				x: window.outerWidth + 380,
				duration: 200,
			}}
			transitionsPositionerOut={{
				x: window.outerWidth + 380,
				duration: 200,
			}}
		>
			{#snippet trigger()}
				<SettingsIcon />
			{/snippet}
			{#snippet content()}
				<Settings bind:openState />
			{/snippet}
		</Modal>
	{/snippet}
</AppBar>
