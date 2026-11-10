<script lang="ts">
	import Navbar from "$components/Navbar.svelte";
	import BlockEditor from "$components/BlockEditor.svelte";
	import { Modal } from "@skeletonlabs/skeleton-svelte";
	import ExperimentRunner from "./components/ExperimentRunner.svelte";
	import { SpeedState } from "$lib/speed_state.js";
	import { LengthState } from "$lib/length_state.js";
	import { Settings } from "$lib/settings_state.js";
    import { invoke } from "@tauri-apps/api/core";
    import { ExperimentIteration } from "$lib/state_machine.js";
	import { Blocks, total_trials } from "$lib/blocks_state.js";
	import {
		load_instructions,
		type InstructionSet,
	} from "$lib/instructions.js";

	let openState = $state(false);

	$inspect(ExperimentIteration.current);

	let trials = $derived(total_trials(Blocks.current));
	// Mirror the hidden `required` speed/distance inputs so the reason Start is blocked is
	// visible, since a `display:none` field can't be scrolled to or highlighted by the browser.
	let missing_speed = $derived(!Number(SpeedState.current));
	let missing_distance = $derived(!Number(LengthState.current));

	// The instructions are files the researcher edits while the app runs. They are read here so
	// a broken folder shows up before anyone presses Start, and read again at the start itself
	// so the run shows the pages as they are on disk at that moment.
	let instructions: InstructionSet | undefined = $state();
	let instructions_error: string | undefined = $state();

	async function refresh_instructions(): Promise<InstructionSet | undefined> {
		try {
			instructions = await load_instructions(Settings.current.lang);
			instructions_error = undefined;
		} catch (e) {
			instructions = undefined;
			instructions_error = String(e);
		}
		return instructions;
	}

	$effect(() => {
		// Rerun when the language changes, every language has a folder of its own.
		Settings.current.lang;
		refresh_instructions();
	});

	async function start_experiment() {
		// An empty block configuration has no trials to run.
		if (trials === 0) {
			return;
		}
		// Without valid instructions the subject would be dropped straight into the trials.
		if (!(await refresh_instructions())) {
			return;
		}
		await invoke("init_logger", { name: Settings.current.subject_name });
		openState = true;
	}
</script>

<Navbar />
<main class="container m-auto px-4">
	<form
		class="mx-auto my-10 w-full max-w-3xl space-y-6 flex flex-col"
		onsubmit={start_experiment}
	>
		<!-- Hidden but still `required`: `class="hidden"` is CSS display:none, not the HTML
		     `hidden` attribute, so these stay in constraint validation and block Start until
		     the speed is calibrated and a distance is set (configured in the settings drawer). -->
		<label class="label hidden">
			<span class="label-text">Walking Speed in km/h</span>
			<input
				type="number"
				class="input"
				placeholder="Speed"
				step="0.01"
				required
				bind:value={SpeedState.current}
			/>
		</label>
		<label class="label hidden">
			<span class="label-text">Total walking distance per trial in m</span>
			<input
				type="number"
				class="input"
				placeholder="Distance in m"
				required
				bind:value={LengthState.current}
			/>
		</label>
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<label class="label">
				<span class="label-text">Subject Name</span>
				<input
					type="text"
					class="input"
					placeholder="Subject name"
					required
					bind:value={Settings.current.subject_name}
				/>
			</label>
			<label class="label">
				<span class="label-text">Study Name</span>
				<input
					type="text"
					class="input"
					placeholder="Study name"
					required
					bind:value={Settings.current.study_name}
				/>
			</label>
		</div>

		<BlockEditor />

		<div class="flex flex-col gap-2">
			<button
				class="btn preset-filled-primary-500 dark:preset-filled-primary-500"
				type="submit"
				disabled={trials === 0 || !!instructions_error}>Start</button
			>
			{#if instructions_error}
				<div class="text-sm text-error-600-400">
					<p class="font-semibold">The instructions cannot be shown.</p>
					<p>{instructions_error}</p>
					<button
						type="button"
						class="btn btn-sm preset-tonal mt-2"
						onclick={refresh_instructions}>Check again</button
					>
				</div>
			{:else if instructions}
				<p class="text-sm opacity-75">
					Instructions: {instructions.pages.length}
					{instructions.pages.length === 1 ? "page" : "pages"} from
					<code class="code">{instructions.dir}</code>
				</p>
				{#each instructions.warnings as warning}
					<p class="text-sm text-warning-600-400">{warning}</p>
				{/each}
			{/if}
			{#if trials === 0}
				<p class="text-sm opacity-75">
					No trials configured yet — add a block above to enable the
					experiment.
				</p>
			{/if}
			{#if missing_speed || missing_distance}
				<p class="text-sm text-warning-600-400">
					{#if missing_speed && missing_distance}
						Calibrate the walking speed and set the distance per trial
						in the settings before starting.
					{:else if missing_speed}
						Calibrate the walking speed in the settings before
						starting.
					{:else}
						Set the walking distance per trial in the settings before
						starting.
					{/if}
				</p>
			{/if}
		</div>
		<Modal
			open={openState}
			onOpenChange={(e) => (openState = e.open)}
			contentBase="card bg-surface-100-900 space-y-4 shadow-xl min-w-screen min-h-screen"
			backdropClasses="backdrop-blur-sm"
		>
			{#snippet content()}
				<!-- `instructions` is always set here, the start is refused without it. -->
				<ExperimentRunner
					bind:openState
					length={LengthState.current as number}
					instructions={instructions!}
				/>
			{/snippet}
		</Modal>
	</form>
</main>
