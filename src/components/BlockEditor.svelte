<script lang="ts">
	import {
		Blocks,
		next_block_settings,
		shows_image,
		total_trials,
		type Block,
		type BlockKind,
		type SpeedVariations,
	} from "$lib/blocks_state.js";
	import type { TimeMod } from "$lib/jitter.js";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import Plus from "@lucide/svelte/icons/plus";
	import GripVertical from "@lucide/svelte/icons/grip-vertical";
	import SlidersHorizontal from "@lucide/svelte/icons/sliders-horizontal";

	const speed_variations: [keyof SpeedVariations, string][] = [
		["very_slow", "-20%"],
		["slow", "-10%"],
		["fast", "10%"],
		["very_fast", "20%"],
	];

	// The block whose settings are open. It is an index like the rest of this list, so anything
	// that moves blocks around closes it rather than leaving it on a different block.
	let expanded = $state<number | null>(null);

	function add_block() {
		Blocks.current = [
			...Blocks.current,
			{
				kind: "stimulus",
				trials: 10,
				settings: next_block_settings(Blocks.current),
			},
		];
	}

	function remove_block(i: number) {
		expanded = null;
		Blocks.current = Blocks.current.filter((_, j) => j !== i);
	}

	// `kind` discriminates the block's shape, so switching kind rebuilds the object. The trials
	// count and the settings are carried over between the trial kinds so toggling between them
	// does not reset them.
	function set_kind(i: number, kind: BlockKind) {
		const cur = Blocks.current[i];
		const trials = cur.kind === "pause" ? 10 : cur.trials;
		const settings =
			cur.kind === "pause"
				? next_block_settings(Blocks.current)
				: cur.settings;
		const next: Block =
			kind === "pause" ? { kind } : { kind, trials, settings };
		if (kind === "pause" && expanded === i) {
			expanded = null;
		}
		Blocks.current = Blocks.current.map((b, j) => (j === i ? next : b));
	}

	function move(from: number, to: number) {
		if (from === to) {
			return;
		}
		expanded = null;
		const next = [...Blocks.current];
		const [moved] = next.splice(from, 1);
		next.splice(to, 0, moved);
		Blocks.current = next;
	}

	// Pointer based drag reordering. Native HTML5 drag-and-drop is unreliable on the
	// WebKitGTK webview Tauri uses on Linux, so we drive the reorder from pointer events,
	// which behave the same across every platform's webview. `rows[i]` is the card element at
	// position `i`; the list is unkeyed so a card's index tracks its slot, and we reorder live
	// as the pointer crosses the midpoint of another card.
	let dragging = $state<number | null>(null);
	let rows: HTMLElement[] = [];

	function row_at(y: number): number {
		for (let j = 0; j < rows.length; j++) {
			const rect = rows[j].getBoundingClientRect();
			if (y < rect.top + rect.height / 2) {
				return j;
			}
		}
		return rows.length - 1;
	}

	function on_pointer_down(e: PointerEvent, i: number) {
		e.preventDefault();
		dragging = i;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}

	function on_pointer_move(e: PointerEvent) {
		if (dragging === null) {
			return;
		}
		const target = row_at(e.clientY);
		if (target !== dragging) {
			move(dragging, target);
			dragging = target;
		}
	}

	function on_pointer_up(e: PointerEvent) {
		if (dragging === null) {
			return;
		}
		(e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
		dragging = null;
	}
</script>

<div class="space-y-3">
	<div class="flex items-baseline justify-between">
		<h3 class="h3">Block sequence</h3>
		<span class="opacity-75">
			{Blocks.current.length} blocks · {total_trials(Blocks.current)} trials
		</span>
	</div>
	<p class="text-sm opacity-75">
		A <b>stimulus</b> block shows an emotional image on every trial, a
		<b>static stimulus</b> block does the same without any walking, a
		<b>neutral</b> block shows a fixation cross in the image's place, and a
		<b>pause</b> is a break between blocks. Drag the handle to reorder, the
		sliders open a block's own settings.
	</p>

	<ul class="space-y-2">
		{#each Blocks.current as block, i (i)}
			<li
				bind:this={rows[i]}
				class="rounded border border-surface-500/30 bg-surface-50-950 p-3"
				class:opacity-40={dragging === i}
			>
				<div class="flex items-center gap-3">
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<span
						class="cursor-grab touch-none select-none opacity-60 hover:opacity-100"
						aria-label="Drag to reorder block {i + 1}"
						onpointerdown={(e) => on_pointer_down(e, i)}
						onpointermove={on_pointer_move}
						onpointerup={on_pointer_up}
					>
						<GripVertical size={18} />
					</span>

					<span class="w-16 font-bold opacity-70">#{i + 1}</span>

					<select
						class="select w-32"
						value={block.kind}
						onchange={(e) =>
							set_kind(i, e.currentTarget.value as BlockKind)}
					>
						<option value="stimulus">Stimulus</option>
						<option value="static_stimulus">Static Stimulus</option>
						<option value="neutral">Neutral</option>
						<option value="pause">Pause</option>
					</select>

					{#if block.kind !== "pause"}
						<input
							type="number"
							class="input w-24"
							placeholder="Trials"
							min="0"
							step="1"
							required
							bind:value={block.trials}
						/>
						<span class="whitespace-nowrap opacity-75">trials</span>
					{:else}
						<span class="italic opacity-60">break</span>
					{/if}

					<div class="ml-auto flex items-center gap-1">
						{#if block.kind !== "pause"}
							<button
								type="button"
								class="btn-icon btn-icon-sm"
								aria-label="Settings of block {i + 1}"
								aria-expanded={expanded === i}
								onclick={() =>
									(expanded = expanded === i ? null : i)}
							>
								<SlidersHorizontal size={16} />
							</button>
						{/if}
						<button
							type="button"
							class="btn-icon btn-icon-sm"
							aria-label="Remove block {i + 1}"
							onclick={() => remove_block(i)}
						>
							<Trash2 size={16} />
						</button>
					</div>
				</div>

				{#if block.kind !== "pause" && expanded === i}
					<div
						class="mt-3 grid grid-cols-1 gap-4 border-t border-surface-500/30 pt-3 sm:grid-cols-2"
					>
						{#if shows_image(block.kind)}
							<div class="space-y-1">
								<span class="label-text">Rating categories</span>
								<label class="flex items-center space-x-2">
									<input
										class="checkbox"
										type="checkbox"
										bind:checked={block.settings.rating.valence}
									/>
									<p>Valence</p>
								</label>
								<label class="flex items-center space-x-2">
									<input
										class="checkbox"
										type="checkbox"
										bind:checked={block.settings.rating.arousal}
									/>
									<p>Arousal</p>
								</label>
							</div>
						{/if}
						{#if block.kind !== "static_stimulus"}
							<div class="space-y-1">
								<span class="label-text">Speed variations</span>
								{#each speed_variations as [speed, label] (speed)}
									<label class="flex items-center space-x-2">
										<input
											class="checkbox"
											type="checkbox"
											bind:checked={block.settings.speeds[speed]}
										/>
										<p>{label}</p>
									</label>
								{/each}
							</div>
						{/if}
						{@render timing("Baseline", block.settings.baseline)}
						{@render timing("Stimulus", block.settings.stimulus)}
					</div>
				{/if}
			</li>
		{/each}
	</ul>

	<button type="button" class="btn preset-tonal" onclick={add_block}>
		<Plus size={16} />
		<span>Add block</span>
	</button>
</div>

<!-- Not `required`: these sit inside the Start form, where a collapsed panel could not show why
     it blocks, and an empty field simply counts as 0, see `jittered_duration`. -->
{#snippet timing(label: string, mod: TimeMod)}
	<div class="space-y-1">
		<span class="label-text">{label} in s</span>
		<label class="flex items-center space-x-2">
			<input
				type="number"
				class="input w-24"
				placeholder="Duration"
				min="0"
				step="any"
				bind:value={mod.duration}
			/>
			<p>Duration</p>
		</label>
		<label class="flex items-center space-x-2">
			<input
				type="number"
				class="input w-24"
				placeholder="Jitter"
				min="0"
				step="any"
				bind:value={mod.jitter}
			/>
			<p>Jitter</p>
		</label>
	</div>
{/snippet}
