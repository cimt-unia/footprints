<script lang="ts">
	import { LengthState } from "$lib/length_state.js";
	import TestRunner from "$routes/experiment/components/TestRunner.svelte";
	import { Modal } from "@skeletonlabs/skeleton-svelte";
	import { _ } from "svelte-i18n";

	interface Props {
		cb: () => Promise<void>;
	}

	const { cb }: Props = $props();

	let step = $state(0);
	let test_open = $state(false);

	async function onKeyDown(e: KeyboardEvent) {
		if (test_open) {
			return;
		}
		switch (e.key) {
			case "Enter": {
				if (step == 4) {
					// start exp
					await cb();
				} else {
					step++;
				}
				break;
			}
			case "T":
			case "t": {
				if (step == 4) {
					test_open = true;
				}
				break;
			}
			case "Backspace": {
				step = Math.max(0, --step);
				break;
			}
		}
	}
</script>

<div class="flex flex-col m-auto max-w-2xl text-xl justify-between">
	<h1 class="h1 mb-10 mt-10">{$_("instructions.h1")}</h1>
	{#if step === 0}
		<b>{$_("instructions.welcome.h2")}</b>
		<p class="mt-2">{$_("instructions.welcome.p1")}</p>
		<p class="mt-4">{$_("instructions.welcome.p2")}</p>
		<p class="mt-4">{$_("instructions.welcome.p3")}</p>
		<p class="mt-4">{$_("instructions.welcome.p4")}</p>
	{:else if step === 1}
		<h2 class="h2">{$_("instructions.valence.h2")}</h2>

		<p class="mt-4">{$_("instructions.valence.p1")}</p>
		<p class="mt-4">{@html $_("instructions.valence.p2")}</p>
		<p class="mt-4">{$_("instructions.valence.p3")}</p>
		<p class="mt-4">{$_("instructions.valence.p4")}</p>
		<p class="mt-4">{$_("instructions.valence.p5")}</p>
	{:else if step === 2}
		<h2 class="h2">{$_("instructions.arousal.h2")}</h2>

		<p class="mt-4">{$_("instructions.arousal.p1")}</p>
		<p class="mt-4">{$_("instructions.arousal.p2")}</p>
		<p class="mt-4">{$_("instructions.arousal.p3")}</p>
		<p class="mt-4">{$_("instructions.arousal.p4")}</p>
	{:else if step === 3}
		<h2 class="h2">{$_("instructions.procedure.h2")}</h2>

		<h4 class="h4 mt-4">{$_("instructions.procedure.phase_1.h2")}</h4>
		<p class="mt-4 mb-4">{@html $_("instructions.procedure.phase_1.p1")}</p>
		<h4 class="h4 mt-4">{$_("instructions.procedure.phase_2.h2")}</h4>
		<p class="mt-4 mb-4">{@html $_("instructions.procedure.phase_2.p1")}</p>
		<h4 class="h4">{$_("instructions.procedure.phase_3.h2")}</h4>
		<p class="mt-4">{@html $_("instructions.procedure.phase_3.p1")}</p>
		<p class="mt-4 mb-4">{@html $_("instructions.procedure.phase_3.p2")}</p>
		<h4 class="h4">{$_("instructions.procedure.phase_4.h2")}</h4>
		<p class="mt-4 mb-4">{@html $_("instructions.procedure.phase_4.p1")}</p>
	{:else if step === 4}
		<h4 class="h4">{$_("instructions.trigger.h2")}</h4>
		<p class="mt-4 mb-4">
			{$_("instructions.trigger.p1")}
		</p>
		<b>
			{$_("instructions.trigger.b")}
		</b>
	{/if}
</div>
<Modal
	open={test_open}
	onOpenChange={(e) => (test_open = e.open)}
	contentBase="card bg-surface-100-900 space-y-4 shadow-xl min-w-screen min-h-screen"
	backdropClasses="backdrop-blur-sm"
>
	{#snippet content()}
		<TestRunner
			bind:openState={test_open}
			length={LengthState.current as number}
		/>
	{/snippet}
</Modal>

<svelte:window on:keydown|preventDefault={onKeyDown} />
