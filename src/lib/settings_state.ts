import { PersistedState } from "runed";

interface Settings {
	lang: "de" | "en";
	show_countdown: boolean;
	sound_cue: boolean;
	subject_name: string;
	study_name: string;
	task_instructions: any;
	very_slow: boolean;
	slow: boolean;
	fast: boolean;
	very_fast: boolean;
	iterations: number;
	rating: {
		arousal: boolean;
		valence: boolean;
	}
}

export const Settings = new PersistedState("settings", default_settings());

function default_settings(): Settings {
	return {
		lang: "de",
		show_countdown: false,
		sound_cue: false,
		subject_name: "Subject_1",
		study_name: "Study",
		task_instructions: "",
		very_slow: false,
		slow: false,
		fast: false,
		very_fast: false,
		iterations: 2,
		rating: {
			arousal: true,
			valence: true
		}
	}
}
