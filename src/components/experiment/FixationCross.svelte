<script lang="ts">
	interface Props {
		size?: number;
		thickness?: number;
		color?: string;
	}

	const { size = 20, thickness = 2, color = "black" }: Props = $props();
</script>

<div
	class="fixation-cross"
	style="--cross-size: {size}px; --cross-thickness: {thickness}px; --cross-color: {color};"
></div>

<style>
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
