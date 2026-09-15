import { invoke } from "@tauri-apps/api/core";
import { Settings } from "$lib/settings_state.js";

/**
 * Signals the subject to start walking, with a sound and/or a green flash of `target`
 * depending on the settings.
 */
export function go_cue(target?: HTMLElement) {
	if (Settings.current.sound_cue) {
		invoke("play_sound");
	}
	if (Settings.current.visual_cue) {
		target?.animate([{ offset: 0.5, backgroundColor: "green" }], {
			duration: 600,
			easing: "ease-out",
		});
	}
}
