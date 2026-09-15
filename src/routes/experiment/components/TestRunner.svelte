<script lang="ts">
	import Ongoing from "$components/experiment/Ongoing.svelte";
	import Rating from "$components/experiment/Rating.svelte";
	import Confirm from "$components/experiment/Confirm.svelte";
	import Pause from "$components/experiment/Pause.svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { onDestroy } from "svelte";
	import Baseline from "$components/experiment/Baseline.svelte";
	import { fly } from "svelte/transition";
	import { quintOut } from "svelte/easing";
	import {
		create_state_machine,
		ExperimentIteration,
	} from "$lib/state_machine.js";
	import { TrialImages } from "$lib/trial_images.svelte.js";
	import { SpeedState } from "$lib/speed_state.js";
	import { Settings } from "$lib/settings_state.js";
	import { build_trial_plan } from "$lib/blocks_state.js";
	import { balanced_durations, speed_conditions } from "$lib/durations.js";

	interface Experiment {
		openState: boolean;
		length: number;
	}

	let { openState = $bindable(), ...data }: Experiment = $props();

	// The test run is always a single short block with a stimulus, independent of the
	// configured blocks.
	const plan = build_trial_plan([{ kind: "stimulus", trials: 3 }]);

	// Owns the stimulus image of the running trial and resets the pool on construction.
	const images = new TrialImages(plan);

	const StateMap = {
		// A test run is started from inside the instructions, it never shows them itself.
		instruction: undefined,
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
		// This was just the test run reset counter
		ExperimentIteration.current = 0;
		openState = false;
	}

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
		"test",
	);
	// The full screen wrapper the visual go cue flashes.
	let cue_target: HTMLDivElement | undefined = $state();

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
				marker_block_type="test"
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
