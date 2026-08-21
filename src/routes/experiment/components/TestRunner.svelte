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
	import { LsLEvent, publish_event } from "$lib/lsl.js";
	import {
		create_state_machine,
		ExperimentIteration,
	} from "$lib/state_machine.js";
	import { resource } from "runed";
	import { SpeedState } from "$lib/speed_state.js";
	import { Settings } from "$lib/settings_state.js";
	import { build_trial_plan } from "$lib/blocks_state.js";
	import { balanced_durations, speed_conditions } from "$lib/durations.js";

	interface Experiment {
		openState: boolean;
		length: number;
	}

	interface Image {
		name: string;
		valence: "Low" | "High";
		arousal: "Low" | "High";
		data: any;
	}

	let { openState = $bindable(), ...data }: Experiment = $props();

	// The test run is always a single short block with a stimulus, independent of the
	// configured blocks.
	const plan = build_trial_plan([{ kind: "stimulus", trials: 3 }]);

	invoke("reset_images");

	const img_data = resource(
		() => ExperimentIteration.current,
		async (iteration, _prev, { data, onCleanup }) => {
			onCleanup(() => {
				if (data?.url) {
					URL.revokeObjectURL(data.url);
				}
			});

			if (plan[iteration]?.kind !== "stimulus") {
				return undefined;
			}

			const img: Image = await invoke("get_image");
			let buffer = new Uint8Array(img.data).buffer;
			const blob = new Blob([buffer], { type: "image/webp" });
			return {
				name: img.name,
				valence: img.valence,
				arousal: img.arousal,
				url: URL.createObjectURL(blob),
			};
		},
	);

	const StateMap = {
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

	const experiment_state_machine = create_state_machine(close, plan);
	let signal = $derived(experiment_state_machine.current === "go");

	const durations = balanced_durations(
		speed_conditions(
			data.length,
			SpeedState.current as number,
			Settings.current,
		),
		plan,
	);
	let index = $derived(
		Math.min(ExperimentIteration.current, durations.length - 1),
	);

	let State = $derived(StateMap[experiment_state_machine.current]);

	onDestroy(async () => {
		experiment_state_machine.send("cancel");
		if (img_data.current?.url) {
			URL.revokeObjectURL(img_data.current.url);
		}
		await publish_event(LsLEvent.Idle);
	});
</script>

{#key State}
	{#if State}
		<div
			class="absolute min-w-screen min-h-screen p-4"
			class:flash-background={signal}
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
				running={true}
				duration={durations[index]}
				state_machine={experiment_state_machine}
				img_valence={img_data.current?.valence}
				img_arousal={img_data.current?.arousal}
				img_name={img_data.current?.name}
				img_url={img_data.current?.url}
			/>
		</div>
	{/if}
{/key}

<style>
	@keyframes flash {
		50% {
			background-color: green;
		}
	}

	.flash-background {
		animation: flash 600ms ease-out 1;
	}
</style>
