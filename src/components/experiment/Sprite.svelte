<script lang="ts">
	import { Settings } from "$lib/settings_state.js";
	import type { MyEvents, MyStates } from "$lib/state_machine.js";
	import { invoke } from "@tauri-apps/api/core";
	import type { FiniteStateMachine } from "runed";
	import { onMount } from "svelte";

	interface Props {
		w: number;
		y: number;
		duration: {
			name: string;
			time: number;
		};
		state_machine: FiniteStateMachine<MyStates, MyEvents>;
	}

	let props: Props = $props();
	let tracker: HTMLDivElement | undefined = $state();
	// svelte-ignore non_reactive_update
	let animation: Animation;

	// const names = ["one"];
	const names = ["one", "two", "three", "four", "five", "six"].reverse();

	let current = $state(0);
	let left = $state(false);
	let waiting_left = $state(false);
	let waiting_right = $state(false);
	let name = $derived(names[current]);

	function sleep(time: number) {
		return new Promise((resolve) => setTimeout(resolve, time));
	}

	$effect(() => {
		animation = tracker!.animate(
			[{ right: `${props.w / 2 - 160}px` }, { right: "10%" }],
			{
				duration: props.duration.time / 4,
				easing: "linear",
			},
		);
		let animation2: Animation | undefined = undefined;
		let animation3: Animation | undefined = undefined;
		animation.play();
		animation.onfinish = async () => {
			waiting_right = true;
			await sleep(3000);
			if (Settings.current.sound_cue) {
				invoke("play_sound");
			}
			waiting_right = false;
			left = true;
			animation2 = tracker!.animate(
				[
					{ left: `${props.w - 320 - 0.1 * props.w}px` },
					{ left: "10%" },
				],
				{
					duration: props.duration.time / 2,
					easing: "linear",
				},
			);
			animation2.play();
			animation2.onfinish = async () => {
				waiting_left = true;
				await sleep(3000);
				if (Settings.current.sound_cue) {
					invoke("play_sound");
				}
				waiting_left = false;
				left = false;
				animation3 = tracker!.animate(
					[{ left: "10%" }, { left: `${props.w / 2 - 160}px` }],
					{
						duration: props.duration.time / 4,
						easing: "linear",
					},
				);
				animation3.play();
				animation3.onfinish = () => {
					tracker!.style.left = `${props.w / 2 - 160}px`;
					props.state_machine.send("g_fin");
				};
			};
		};
		return () => {
			animation.cancel();
			animation2?.cancel();
			animation3?.cancel();
		};
	});

	onMount(() => {
		const interval = setInterval(() => {
			current = (current + 1) % names.length;
		}, 500);

		return () => clearInterval(interval);
	});
</script>

<div class="scene-container" style="top: {props.y}px;">
	<img
		src="/sprites/Buildings/Red/Tower.png"
		class="boundary-image start-point"
		alt="left"
		style="absolute"
	/>
	<div
		data-sevenup="{name}.png"
		bind:this={tracker}
		class="{left ? 'left' : ''} {waiting_left
			? 'waiting_left'
			: ''} {waiting_right ? 'waiting_right ' : ''} "
	></div>
	<img
		src="/sprites/Buildings/Red/Tower.png"
		class="boundary-image end-point"
		alt="right"
		style="absolute"
	/>
</div>

<style>
	.waiting_right {
		right: 10%;
	}

	.waiting_left {
		left: 10%;
	}

	.scene-container {
		position: relative;
		width: 100%;
		/* Set height to match your sprite */
		height: 320px;
	}

	/*
	  Base styles for the boundary images.
	  They are 'absolute' so they don't affect other elements.
	  'z-index: 1' puts them *behind* the sprite.
	*/
	.boundary-image {
		position: absolute;
		top: 0;
	}
	.start-point {
		left: 10%;
	}
	.end-point {
		right: 10%;
	}
	.left {
		-webkit-transform: scaleX(-1);
		transform: scaleX(-1);
	}
	[data-sevenup] {
		background-image: url("/sprites/Units/Blue/Lancer/Lancer_Run.png");
		background-size: 1920px 320px;
		position: absolute;
	}
	[data-sevenup="one.png"] {
		width: 320px;
		height: 320px;
		background-position: 0px 0px;
	}
	[data-sevenup="two.png"] {
		width: 320px;
		height: 320px;
		background-position: -320px 0px;
	}
	[data-sevenup="three.png"] {
		width: 320px;
		height: 320px;
		background-position: -640px 0px;
	}
	[data-sevenup="four.png"] {
		width: 320px;
		height: 320px;
		background-position: -960px 0px;
	}
	[data-sevenup="five.png"] {
		width: 320px;
		height: 320px;
		background-position: -1280px 0px;
	}
	[data-sevenup="six.png"] {
		width: 320px;
		height: 320px;
		background-position: -1600px 0px;
	}
</style>
