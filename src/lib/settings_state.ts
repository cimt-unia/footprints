import { PersistedState } from "runed";

interface Settings {
	lang: "de" | "en";
	show_countdown: boolean;
	sound_cue: boolean;
	visual_cue: boolean;
	subject_name: string;
	study_name: string;
	task_instructions: any;
	iterations: number;
}

export const Settings = new PersistedState("settings", default_settings());

function default_settings(): Settings {
	return {
		lang: "de",
		show_countdown: false,
		sound_cue: false,
		visual_cue: true,
		subject_name: "Subject_1",
		study_name: "Study",
		task_instructions: "",
		iterations: 2,
	}
}
