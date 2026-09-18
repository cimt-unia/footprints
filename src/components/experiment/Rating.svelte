<script lang="ts">
	import type { ExperimentStateProps } from "./types.js";
	import { Settings } from "$lib/settings_state.js";
	import { publish_event, ratingMarker } from "$lib/lsl.js";

	const props: ExperimentStateProps = $props();

	let valence_rating = $state(
		Settings.current.rating.valence ? 4 : undefined,
	);
	let arousal_rating = $state(
		Settings.current.rating.arousal ? 4 : undefined,
	);
	let step = $state(Settings.current.rating.valence ? 0 : 1);

	async function onKeyDown(e: KeyboardEvent) {
		switch (e.key) {
			case "1": {
				if (step === 0) {
					valence_rating = Number(e.key);
				} else {
					arousal_rating = Number(e.key);
				}
				break;
			}
			case "2": {
				if (step === 0) {
					valence_rating = Number(e.key);
				} else {
					arousal_rating = Number(e.key);
				}
				break;
			}
			case "3": {
				if (step === 0) {
					valence_rating = Number(e.key);
				} else {
					arousal_rating = Number(e.key);
				}
				break;
			}
			case "4": {
				if (step === 0) {
					valence_rating = Number(e.key);
				} else {
					arousal_rating = Number(e.key);
				}
				break;
			}
			case "5": {
				if (step === 0) {
					valence_rating = Number(e.key);
				} else {
					arousal_rating = Number(e.key);
				}
				break;
			}
			case "6": {
				if (step === 0) {
					valence_rating = Number(e.key);
				} else {
					arousal_rating = Number(e.key);
				}
				break;
			}
			case "7": {
				if (step === 0) {
					valence_rating = Number(e.key);
				} else {
					arousal_rating = Number(e.key);
				}
				break;
			}
			case "Enter": {
				await cont();
			}
		}
	}
	// `cont` publishes a marker before it advances, and that is an IPC round trip. A second
	// Enter arriving inside that window would re-enter with the step unchanged and publish the
	// same rating twice, so only one confirmation may be in flight.
	let confirming = false;
	// The screen stays mounted for the duration of its out transition, so it still sees key
	// presses after the last rating was sent. The state machine ignores the repeated `rated`,
	// but the marker would go out again, hence the latch.
	let done = false;

	async function cont() {
		if (confirming || done) {
			return;
		}
		confirming = true;
		try {
			if (step == 0) {
				await publish_event(
					ratingMarker(
						props.current_trial,
						{ state: "rating_valence", rating: valence_rating! },
						{
							image_id: props.img_id,
							speed: props.duration.kind,
							type: props.marker_type,
						},
					),
				);
				if (Settings.current.rating.arousal) {
					step++;
				} else {
					done = true;
					props.state_machine.send("rated", {
						valence: valence_rating,
						arousal: arousal_rating,
					});
				}
			} else {
				await publish_event(
					ratingMarker(
						props.current_trial,
						{ state: "rating_arousal", rating: arousal_rating! },
						{
							image_id: props.img_id,
							speed: props.duration.kind,
							type: props.marker_type,
						},
					),
				);
				done = true;
				props.state_machine.send("rated", {
					valence: valence_rating,
					arousal: arousal_rating,
				});
			}
		} finally {
			confirming = false;
		}
	}
</script>

<div class="container flex m-auto items-center justify-center h-screen">
	<div class="mx-auto w-full basis-2/3">
		<img class="m-auto w-full" src={props.img_url} alt="stimulus" />
	</div>
	{#if step === 0}
		<form
			class="mx-auto w-full m-auto flex flex-col basis-1/3"
			onsubmit={() => {
				cont();
			}}
		>
			<h2 class="h2 m-auto">Valence</h2>
			<p class="mt-2 m-auto">Bewerten Sie das Bild von 1-7.</p>
			<p class="mt-2 m-auto">Mit enter bestätigen.</p>
		</form>
	{:else}
		<form
			class="mx-auto w-full m-auto flex flex-col basis-1/3"
			onsubmit={() => cont()}
		>
			<h2 class="h2 m-auto">Arousal</h2>
			<p class="mt-2 m-auto">Bewerten Sie das Bild von 1-7.</p>
			<p class="mt-2 m-auto">Mit enter bestätigen.</p>
		</form>
	{/if}
</div>

<svelte:window on:keydown|preventDefault={onKeyDown} />
