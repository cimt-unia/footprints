<script lang="ts">
	import Play from "@lucide/svelte/icons/play";
	import CircleStop from "@lucide/svelte/icons/circle-stop";
	import { Modal } from "@skeletonlabs/skeleton-svelte";
    import { invoke } from "@tauri-apps/api/core";

	interface Props {
		steps: number;
		length: number;
		openState: boolean;
		speed: number | null;
	}

	let {
		steps,
		length,
		speed = $bindable(),
		openState = $bindable(),
	}: Props = $props();
	const size = 20;
	const thickness = 2;
	const color = "black";
	let step = $state(1);
	let running = $state(false);
	let open_decider = $state(false);
	let elapsed = $state(0);
	let frame: number | undefined = undefined;
	let step_speeds: number[] = $state([]);

	const step_speed = $derived(
		elapsed === 0 ? 0 : (length / (elapsed / 1000)) * 3.6,
	);

	function start() {
		let last_time = performance.now();
		invoke("play_sound");

		frame = requestAnimationFrame(function update(time) {
			frame = requestAnimationFrame(update);
			elapsed += time - last_time;
			last_time = time;
		});
		running = true;
	}

	function stop() {
		if (frame) {
			cancelAnimationFrame(frame);
			frame = undefined;
		}
		running = false;
		open_decider = true;
	}

	function on_key_down(event: KeyboardEvent) {
		if (event.repeat) return;

		if (event.key == " " || event.code == "Space") {
			event.preventDefault();
			if (open_decider) return;
			if (running) {
				stop();
			} else {
				start();
			}
		}
	}

	function discard_step() {
		elapsed = 0;
		open_decider = false;
	}

	function confirm_step() {
		step_speeds.push(step_speed);
		if (step === steps) {
			speed = Number(
				(
					step_speeds.reduce((a, b) => a + b) / step_speeds.length
				).toFixed(2),
			);
			openState = false;
		}
		step += 1;
		elapsed = 0;
		open_decider = false;
	}
</script>

<svelte:window on:keydown={on_key_down} />

<div
	class="absolute min-w-screen min-h-screen p-4"
	class:flash-background={running}
>
	<h2 class="h2 pt-3 text-center">Step {step}/{steps}</h2>
	<div class="fixation-cross-container">
		<div
			class="fixation-cross"
			style="--cross-size: {size}px; --cross-thickness: {thickness}px; --cross-color: {color};"
		></div>
	</div>
	<div class="flex mt-5 container m-auto justify-center">
		<Modal
			open={open_decider}
			onOpenChange={(e) => (open_decider = e.open)}
			triggerBase="btn preset-tonal"
			contentBase="card bg-surface-100-900 p-4 space-y-4 shadow-xl max-w-screen-sm"
			backdropClasses="backdrop-blur-sm"
		>
			{#snippet content()}
				<header class="flex justify-between">
					<h2 class="h2">Step results</h2>
				</header>
				<article>
					<p class="opacity-60">
						Time: {(elapsed / 1000).toFixed(2)}s
					</p>
					<p class="opacity-60">
						Length: {length}m
					</p>
					<p class="opacity-60">
						Speed: {step_speed.toFixed(2)}km/h
					</p>
				</article>
				<footer class="flex justify-end gap-4">
					<button
						type="button"
						class="btn preset-tonal"
						onclick={discard_step}>Discard</button
					>
					<button
						type="button"
						class="btn preset-filled"
						onclick={confirm_step}>Confirm</button
					>
				</footer>
			{/snippet}
		</Modal>
		{#if running}
			<button
				type="button"
				class="btn-icon preset-filled-error-500"
				onclick={stop}><CircleStop /></button
			>
		{:else}
			<button
				type="button"
				class="btn-icon preset-filled-success-500"
				onclick={start}><Play /></button
			>
		{/if}
	</div>
</div>

<style>
	@keyframes flash {
		50% {
			background-color: green;
		}
	}

	.flash-background {
		animation: flash 600ms ease-out 1;
	}
	.fixation-cross-container {
		background-color: white;
		display: flex;
		justify-content: center;
		align-items: center;
		width: 80%;
		height: 75vh; /* Or a specific height if you prefer */
		position: relative; /* Or relative, depending on your layout */
		top: 10%;
		left: 10%;
	}

	.fixation-cross {
		width: var(--cross-size);
		height: var(--cross-size);
		position: relative;
	}

	.fixation-cross::before,
	.fixation-cross::after {
		content: "";
		position: absolute;
		background-color: var(--cross-color);
	}

	/* Horizontal line */
	.fixation-cross::before {
		left: 50%;
		top: 50%;
		width: var(--cross-size);
		height: var(--cross-thickness);
		transform: translate(-50%, -50%);
	}

	/* Vertical line */
	.fixation-cross::after {
		left: 50%;
		top: 50%;
		width: var(--cross-thickness);
		height: var(--cross-size);
		transform: translate(-50%, -50%);
	}
</style>
