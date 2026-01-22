import { init, register } from "svelte-i18n";

import de from "./de.json";
import en from "./en.json";
import { Settings } from "$lib/settings_state.js";

register('de', () => Promise.resolve(de));
register('en', () => Promise.resolve(en));

init({
	fallbackLocale: 'de',
	initialLocale: Settings.current.lang,
})

