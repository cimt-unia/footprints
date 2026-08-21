<script lang="ts">
	import {
		Blocks,
		total_trials,
		type Block,
		type BlockKind,
	} from "$lib/blocks_state.js";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import Plus from "@lucide/svelte/icons/plus";
	import GripVertical from "@lucide/svelte/icons/grip-vertical";

	function add_block() {
		Blocks.current = [...Blocks.current, { kind: "stimulus", trials: 10 }];
	}

	function remove_block(i: number) {
		Blocks.current = Blocks.current.filter((_, j) => j !== i);
	}

	// `kind` discriminates the block's shape, so switching kind rebuilds the object. The
	// trials count is carried over between the two trial kinds so toggling stimulus/neutral
	// does not reset it.
	function set_kind(i: number, kind: BlockKind) {
		const cur = Blocks.current[i];
		const trials = cur.kind === "pause" ? 10 : cur.trials;
		const next: Block = kind === "pause" ? { kind } : { kind, trials };
		Blocks.current = Blocks.current.map((b, j) => (j === i ? next : b));
	}

	function move(from: number, to: number) {
		if (from === to) {
			return;
		}
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
		<b>neutral</b> block a fixation cross in its place, and a <b>pause</b> is
		a break between blocks. Drag the handle to reorder.
	</p>

	<ul class="space-y-2">
		{#each Blocks.current as block, i (i)}
			<li
				bind:this={rows[i]}
				class="flex items-center gap-3 rounded border border-surface-500/30 bg-surface-50-950 p-3"
				class:opacity-40={dragging === i}
			>
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

				<button
					type="button"
					class="btn-icon btn-icon-sm ml-auto"
					aria-label="Remove block {i + 1}"
					onclick={() => remove_block(i)}
				>
					<Trash2 size={16} />
				</button>
			</li>
		{/each}
	</ul>

	<button type="button" class="btn preset-tonal" onclick={add_block}>
		<Plus size={16} />
		<span>Add block</span>
	</button>
</div>
