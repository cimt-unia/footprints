import { invoke } from "@tauri-apps/api/core";
import DOMPurify from "dompurify";
import { marked } from "marked";

/** One instruction page, mirrors `InstructionPage` in `instructions.rs`. */
export interface InstructionPage {
	file: string;
	number: number;
	markdown: string;
}

/** The instruction pages of one language, mirrors `InstructionSet` in `instructions.rs`. */
export interface InstructionSet {
	lang: string;
	/** The folder the pages were read from. */
	dir: string;
	pages: InstructionPage[];
	/** Files that were skipped, most likely a typo in the name. */
	warnings: string[];
	/** A digest over the names and contents of the pages. */
	hash: string;
}

/**
 * Reads the instruction pages of `lang` from the study folder. Rejects with a message for the
 * experimenter when the folder is missing or broken, the study cannot start without them.
 */
export async function load_instructions(lang: string): Promise<InstructionSet> {
	return invoke<InstructionSet>("load_instructions", { lang });
}

/**
 * Renders one page. The researcher writes the files, but the webview can reach the backend, so
 * a script in a page must not run: the HTML is sanitized after Markdown has produced it.
 */
export function render_page(page: InstructionPage): string {
	const html = marked.parse(page.markdown, { async: false });
	return DOMPurify.sanitize(html);
}
