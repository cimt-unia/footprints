<script lang="ts">
	import { LengthState } from "$lib/length_state.js";
	import { render_page } from "$lib/instructions.js";
	import TestRunner from "$routes/experiment/components/TestRunner.svelte";
	import { Modal } from "@skeletonlabs/skeleton-svelte";
	import { _ } from "svelte-i18n";
	import type { ExperimentStateProps } from "./types.js";

	// The instructions are a state of their own rather than a part of the baseline, so that the
	// baseline marker is stamped when the fixation cross appears and not when this screen opens.
	const props: ExperimentStateProps = $props();

	// The researcher writes the pages, see `instructions.rs`. The navigation around them is the
	// app's, so a page cannot break it and every study explains the keys the same way.
	const pages = (props.instructions?.pages ?? []).map(render_page);

	let step = $state(0);
	let test_open = $state(false);

	function onKeyDown(e: KeyboardEvent) {
		if (test_open) {
			return;
		}
		switch (e.key) {
			case "Enter": {
				if (step >= pages.length - 1) {
					props.state_machine.send("instructed");
				} else {
					step++;
				}
				break;
			}
			case "Backspace": {
				step = Math.max(0, step - 1);
				break;
			}
			// A practice run can be useful at any point, e.g. right after the procedure is
			// explained, so it is not tied to the last page.
			case "T":
			case "t": {
				test_open = true;
				break;
			}
		}
	}
</script>

<div class="flex flex-col m-auto max-w-2xl min-h-[90vh] text-xl">
	<article class="instruction-page mt-10 grow">
		{@html pages[step] ?? ""}
	</article>
	<footer class="flex justify-between mt-10 mb-4 text-base opacity-75">
		<span>
			{step < pages.length - 1
				? $_("instructions.nav.next")
				: $_("instructions.nav.start")}
			{#if step > 0}
				· {$_("instructions.nav.back")}
			{/if}
			· {$_("instructions.nav.test")}
		</span>
		{#if pages.length > 1}
			<span>{step + 1} / {pages.length}</span>
		{/if}
	</footer>
</div>
<Modal
	open={test_open}
	onOpenChange={(e) => {
		test_open = e.open;
		// A test run shares the trial counter and the image pool with the experiment around
		// it, the state machine has to put both back.
		if (!e.open) {
			props.state_machine.send("test_finished");
		}
	}}
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

<style>
	/* Tailwind's reset strips every element down, the rendered Markdown needs its look back. */
	.instruction-page :global(h1) {
		font-size: 2.25rem;
		font-weight: 700;
		margin-bottom: 1.5rem;
	}
	.instruction-page :global(h2) {
		font-size: 1.75rem;
		font-weight: 700;
		margin: 1.5rem 0 1rem;
	}
	.instruction-page :global(h3),
	.instruction-page :global(h4) {
		font-size: 1.35rem;
		font-weight: 600;
		margin: 1.25rem 0 0.75rem;
	}
	.instruction-page :global(p),
	.instruction-page :global(ul),
	.instruction-page :global(ol),
	.instruction-page :global(blockquote) {
		margin-bottom: 1rem;
	}
	.instruction-page :global(ul) {
		list-style: disc;
		padding-left: 1.5rem;
	}
	.instruction-page :global(ol) {
		list-style: decimal;
		padding-left: 1.5rem;
	}
	.instruction-page :global(blockquote) {
		border-left: 4px solid currentColor;
		padding-left: 1rem;
		opacity: 0.85;
	}
	.instruction-page :global(img) {
		max-width: 100%;
	}
	.instruction-page :global(hr) {
		margin: 1.5rem 0;
	}
</style>
