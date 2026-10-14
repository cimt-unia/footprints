<script lang="ts">
	import Ongoing from "$components/experiment/Ongoing.svelte";
	import Rating from "$components/experiment/Rating.svelte";
	import Confirm from "$components/experiment/Confirm.svelte";
	import Pause from "$components/experiment/Pause.svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { onDestroy } from "svelte";
	import Baseline from "$components/experiment/Baseline.svelte";
	import Instruction from "$components/experiment/Instruction.svelte";
	import { fly } from "svelte/transition";
	import { quintOut } from "svelte/easing";
	import {
		create_state_machine,
		ExperimentIteration,
	} from "$lib/state_machine.js";
	import { TrialImages } from "$lib/trial_images.svelte.js";
	import { SpeedState } from "$lib/speed_state.js";
	import { Settings } from "$lib/settings_state.js";
	import { Blocks, build_trial_plan } from "$lib/blocks_state.js";
	import { balanced_durations, speed_conditions } from "$lib/durations.js";

	interface Experiment {
		openState: boolean;
		length: number;
	}

	let { openState = $bindable(), ...data }: Experiment = $props();

	// Resolved once: the block configuration must not change under a running experiment.
	const plan = build_trial_plan(Blocks.current);

	// Owns the stimulus image of the running trial and resets the pool on construction.
	const images = new TrialImages(plan);

	const StateMap = {
		// The instructions precede the first baseline, a test run skips them.
		instruction: Instruction,
		baseline: Baseline,
		stimulus: Ongoing,
		go: Ongoing,
		rating: Rating,
		confirm: Confirm,
		pause: Pause,
		// The canceled state intentionally renders nothing, see `create_state_machine`.
		canceled: undefined,
	};

	function close() {
		invoke("save_experiment", { study: Settings.current.study_name });
		openState = false;
	}

	// One duration per planned trial, with the speed conditions balanced inside every block.
	// Fixed for the whole run, like the plan it is aligned with.
	const durations = balanced_durations(
		speed_conditions(
			data.length,
			SpeedState.current as number,
			Settings.current,
		),
		plan,
	);
	const experiment_state_machine = create_state_machine(
		close,
		plan,
		durations,
		images,
	);
	// The full screen wrapper the visual go cue flashes.
	let cue_target: HTMLDivElement | undefined = $state();

	// `durations` is index aligned with the plan, so the trial counter indexes it directly.
	// The clamp only matters for the frame after the last trial, before the modal closes.
	let index = $derived(
		Math.min(ExperimentIteration.current, durations.length - 1),
	);

	let State = $derived(StateMap[experiment_state_machine.current]);

	let current_trial = $derived(plan[ExperimentIteration.current]);

	onDestroy(async () => {
		experiment_state_machine.send("cancel");
		images.release();
	});
</script>

{#key State}
	{#if State}
		<div
			class="absolute min-w-screen min-h-screen p-4"
			bind:this={cue_target}
			in:fly={{
				x: 200,
				duration: 500,
				easing: quintOut,
			}}
			out:fly={{
				x: -200,
				duration: 500,
				easing: quintOut,
			}}
		>
			<State
				duration={durations[index]}
				state_machine={experiment_state_machine}
				{current_trial}
				img_id={images.current?.id}
				img_valence={images.current?.valence}
				img_arousal={images.current?.arousal}
				img_name={images.current?.name}
				img_url={images.current?.url}
				{cue_target}
			/>
		</div>
	{/if}
{/key}
