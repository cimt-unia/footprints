<script lang="ts">
	import { LengthState } from "$lib/length_state.js";
	import { BaselineModState } from "$lib/baseline_time_mod.js";
	import { StimulusModState } from "$lib/stimulus_time_mod.js";
	import Lightswitch from "./Lightswitch.svelte";
	import { Settings } from "$lib/settings_state.js";
	import { invoke } from "@tauri-apps/api/core";
	import { SpeedState } from "$lib/speed_state.js";
	import { NumIterations } from "$lib/num_iter_state.js";
	import { locale } from "svelte-i18n";

	let { openState = $bindable() } = $props();

	$inspect(Settings.current);

	async function calibrate() {
		await invoke("open_calibration");
	}
	function save() {
		openState = false;
	}
</script>

<div class="flex flex-col overflow-scroll" style="height: 95vh;">
	<div class="flex justify-between">
		<h3 class="h3">Settings</h3>
		<Lightswitch />
	</div>

	<!--TODO: scrolling-->
	<div>
		<form class="flex-grow space-y-2" onsubmit={save}>
			<div class="space-y-2">
				<span class="label-text text-lg">Some on/off setting:</span>
				<label class="flex items-center space-x-2">
					<input
						class="checkbox"
						type="checkbox"
						bind:checked={Settings.current.show_countdown}
					/>
					<p>Show countdown</p>
				</label>

				<label class="flex items-center space-x-2">
					<input
						class="checkbox"
						type="checkbox"
						bind:checked={Settings.current.sound_cue}
					/>
					<p>Sound cue</p>
				</label>

				<label class="label">
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
				<button type="button" onclick={async () => await calibrate()}
					>Calibrate Speed</button
				>
				<label class="label">
					<span class="label-text">Track length in m</span>
					<input
						type="number"
						class="input"
						placeholder="Length in m"
						required
						bind:value={LengthState.current}
					/>
				</label>
				<label class="label">
					<span class="label-text">Instruction Iterations</span>
					<input
						type="number"
						class="input"
						placeholder="Number of Iterations"
						step="1"
						required
						bind:value={NumIterations.current}
					/>
				</label>
				<label class="label">
					<span class="label-text">Experiment Language</span>
					<select
						class="select"
						bind:value={Settings.current.lang}
						onchange={() => {
							$locale = Settings.current.lang;
						}}
					>
						<option value="de"> Deutsch </option>
						<option value="en"> English </option>
					</select>
				</label>
				<span class="label-text text-lg">Speed variations</span>
				<label class="flex items-center space-x-2">
					<input
						class="checkbox"
						type="checkbox"
						bind:checked={Settings.current.very_slow}
					/>
					<p>-20%</p>
				</label>
				<label class="flex items-center space-x-2">
					<input
						class="checkbox"
						type="checkbox"
						bind:checked={Settings.current.slow}
					/>
					<p>-10%</p>
				</label>
				<label class="flex items-center space-x-2">
					<input
						class="checkbox"
						type="checkbox"
						bind:checked={Settings.current.fast}
					/>
					<p>10%</p>
				</label>
				<label class="flex items-center space-x-2">
					<input
						class="checkbox"
						type="checkbox"
						bind:checked={Settings.current.very_fast}
					/>
					<p>20%</p>
				</label>
				<span class="label-text text-lg">Rating categories</span>
				<label class="flex items-center space-x-2">
					<input
						class="checkbox"
						type="checkbox"
						bind:checked={Settings.current.rating.valence}
					/>
					<p>Valence</p>
				</label>
				<label class="flex items-center space-x-2">
					<input
						class="checkbox"
						type="checkbox"
						bind:checked={Settings.current.rating.arousal}
					/>
					<p>Arousal</p>
				</label>
				<span class="label-text text-lg">Baseline</span>
				<label class="flex items-center space-x-2">
					<input
						type="number"
						class="input"
						placeholder="Baseline duration"
						required
						bind:value={BaselineModState.current.duration}
					/>
					<p>Duration</p>
				</label>
				<label class="flex items-center space-x-2">
					<input
						type="number"
						class="input"
						placeholder="Baseline jitter"
						required
						bind:value={BaselineModState.current.jitter}
					/>
					<p>Jitter</p>
				</label>
				<span class="label-text text-lg">Stimulus</span>
				<label class="flex items-center space-x-2">
					<input
						type="number"
						class="input"
						placeholder="Stimulus duration"
						required
						bind:value={StimulusModState.current.duration}
					/>
					<p>Duration</p>
				</label>
				<label class="flex items-center space-x-2">
					<input
						type="number"
						class="input"
						placeholder="Stimulus jitter"
						required
						bind:value={StimulusModState.current.jitter}
					/>
					<p>Jitter</p>
				</label>
			</div>
		</form>
	</div>
</div>

{#snippet mark(marker: number)}
	{marker.toFixed(1)}
{/snippet}
