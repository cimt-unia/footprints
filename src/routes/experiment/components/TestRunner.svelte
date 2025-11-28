<script lang="ts">
	import Ongoing from "$components/experiment/Ongoing.svelte";
	import Rating from "$components/experiment/Rating.svelte";
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
	import { resource, watch } from "runed";
	import { SpeedState } from "$lib/speed_state.js";
	import { Settings } from "$lib/settings_state.js";

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

	const img_data = resource(
		() => ExperimentIteration.current,
		async (_url, _prev_url, { data, onCleanup }) => {
			onCleanup(() => {
				URL.revokeObjectURL(data);
			});

			const img: Image = await invoke("get_image", {
				init: ExperimentIteration.current === 0,
			});
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
	};

	function close() {
		invoke("save_experiment", { study: Settings.current.study_name });
		// This was just the test run reset counter
		ExperimentIteration.current = 0;
		openState = false;
	}

	const experiment_state_machine = create_state_machine(close, 3);
	let signal = $derived(experiment_state_machine.current === "go");

	function expandArray(arr: any[], n: number) {
		const repeats = Math.ceil(n / arr.length);

		return arr.flatMap((element) => Array(repeats).fill(element));
	}

	let durations = (() => {
		let base = [
			{
				name: "Normal",
				time:
					(data.length / ((SpeedState.current as number) / 3.6)) *
					1000,
			},
		];
		const mask: [boolean, string, number][] = [
			[Settings.current.very_slow, "Very slow", 1.2],
			[Settings.current.slow, "Slow", 1.1],
			[Settings.current.fast, "Fast", 0.9],
			[Settings.current.very_fast, "Very fast", 0.8],
		];
		for (const el of mask) {
			if (el[0]) {
				base.push({
					name: el[1],
					time: base[0].time * el[2],
				});
			}
		}
		let arr = expandArray(base, 3);
		for (var i = arr.length - 1; i > 0; i--) {
			var j = Math.floor(Math.random() * (i + 1));
			var temp = arr[i];
			arr[i] = arr[j];
			arr[j] = temp;
		}
		return arr;
	})();
	let index = $state(0);
	watch(
		() => ExperimentIteration.current,
		() => {
			index += 1;
			if (index >= durations.length) {
				experiment_state_machine.send("cancel");
			}
		},
	);

	let State = $derived(StateMap[experiment_state_machine.current]);

	onDestroy(async () => {
		experiment_state_machine.send("cancel");
		if (img_data.current?.url) {
			URL.revokeObjectURL(img_data.current!.url);
		}
		await publish_event(LsLEvent.Idle);
	});
</script>

{#key State}
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
