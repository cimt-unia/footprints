<script lang="ts">
	import type { Duration } from "$lib/durations.js";
	import { go_cue } from "$lib/go_cue.js";
	import type { MyEvents, MyStates } from "$lib/state_machine.js";
	import type { FiniteStateMachine } from "runed";

	interface Props {
		w: number;
		y: number;
		duration: Duration;
		state_machine: FiniteStateMachine<MyStates, MyEvents>;
		/** Edge length of the square the triangle is drawn into. */
		size?: number;
		color?: string;
		/** The element flashed by the visual go cue at each turnaround. */
		cue_target?: HTMLElement;
	}

	let { size = 120, color = "black", ...props }: Props = $props();
	let tracker: HTMLDivElement | undefined = $state();
	// svelte-ignore non_reactive_update
	let animation: Animation;

	/** True while the marker points towards the left end of the track. */
	let left = $state(false);

	// The marker is positioned by its left edge, so every waypoint is the tick position
	// shifted by half the marker to put its centre on the tick.
	const centre = $derived(props.w / 2 - size / 2);
	const right_tick = $derived(0.9 * props.w - size / 2);
	const left_tick = $derived(0.1 * props.w - size / 2);

	function sleep(time: number) {
		return new Promise((resolve) => setTimeout(resolve, time));
	}

	$effect(() => {
		animation = tracker!.animate(
			[{ left: `${centre}px` }, { left: `${right_tick}px` }],
			{
				duration: props.duration.time / 4,
				easing: "linear",
			},
		);
		let animation2: Animation | undefined = undefined;
		let animation3: Animation | undefined = undefined;
		animation.play();
		animation.onfinish = async () => {
			tracker!.style.left = `${right_tick}px`;
			left = true;
			await sleep(3000);
			go_cue(props.cue_target);
			animation2 = tracker!.animate(
				[{ left: `${right_tick}px` }, { left: `${left_tick}px` }],
				{
					duration: props.duration.time / 2,
					easing: "linear",
				},
			);
			animation2.play();
			animation2.onfinish = async () => {
				tracker!.style.left = `${left_tick}px`;
				left = false;
				await sleep(3000);
				go_cue(props.cue_target);
				animation3 = tracker!.animate(
					[{ left: `${left_tick}px` }, { left: `${centre}px` }],
					{
						duration: props.duration.time / 4,
						easing: "linear",
					},
				);
				animation3.play();
				animation3.onfinish = () => {
					tracker!.style.left = `${centre}px`;
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
</script>

<div
	class="scene-container"
	style="top: {props.y}px; --marker-size: {size}px; --marker-color: {color};"
>
	<!-- The path the subject walks, with a tick at each physical turnaround marker. -->
	<div class="track"></div>
	<div class="tick start-point"></div>
	<div class="tick end-point"></div>
	<div bind:this={tracker} class="marker" style="left: {centre}px;">
		<!-- The triangle points along the direction of travel; it flips once, instantly,
		     one second into the three second turnaround pause. -->
		<div class="facing" class:left>
			<!-- The centroid sits at x=50 so the mirror pivots about the triangle's
			     visual centre: the flip turns it in place instead of shifting it. -->
			<svg viewBox="0 0 100 100" aria-hidden="true">
				<polygon points="27,8 96,50 27,92" />
			</svg>
		</div>
	</div>
</div>

<style>
	.scene-container {
		position: relative;
		width: 100%;
		height: var(--marker-size);
	}

	.track {
		position: absolute;
		top: 50%;
		left: 10%;
		width: 80%;
		height: 2px;
		transform: translateY(-50%);
		background-color: var(--marker-color);
	}

	.tick {
		position: absolute;
		top: 50%;
		width: 6px;
		height: calc(var(--marker-size) * 0.9);
		transform: translate(-50%, -50%);
		background-color: var(--marker-color);
	}

	.start-point {
		left: 10%;
	}

	.end-point {
		left: 90%;
	}

	.marker {
		position: absolute;
		top: 50%;
		width: var(--marker-size);
		height: var(--marker-size);
		transform: translateY(-50%);
	}

	.facing {
		width: 100%;
		height: 100%;
	}

	.facing.left {
		transform: scaleX(-1);
	}

	svg {
		display: block;
		width: 100%;
		height: 100%;
		fill: var(--marker-color);
	}
</style>
