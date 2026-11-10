//! The instruction pages a subject reads before the first trial.
//!
//! They are part of the study, not of the app: the researcher writes them as Markdown files
//! into `<study>/instructions/<lang>/`, one file per page, named `NN_name.md`. The number
//! orders the pages and may have gaps, so a page can be inserted without renaming the rest.
//!
//! The study root lives in the app local data dir for now. Everything that belongs to a study
//! is meant to end up below it, so it can be exported and imported as one folder later.

use log::{info, warn};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

const INSTRUCTIONS_DIR: &str = "instructions";

/// The pages written when there are no instructions at all. They only get the subject into the
/// study, a researcher is expected to replace them.
const DEFAULT_PAGES: &[(&str, &str)] = &[
    (
        "de",
        "# Willkommen\n\nDrücken Sie **Enter**, um die Studie zu starten.\n",
    ),
    ("en", "# Welcome\n\nPress **Enter** to start the study.\n"),
];
const DEFAULT_FILE: &str = "01_default.md";

/// One instruction page.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct InstructionPage {
    /// The file name, which is what a researcher recognises the page by.
    pub file: String,
    pub number: u32,
    pub markdown: String,
}

/// The instruction pages of one language, in the order they are shown.
#[derive(Serialize, Debug)]
pub struct InstructionSet {
    pub lang: String,
    /// The folder the pages were read from, so the experimenter knows which one to edit.
    pub dir: String,
    pub pages: Vec<InstructionPage>,
    /// Files that were skipped. They do not stop the study, but are most likely a typo.
    pub warnings: Vec<String>,
    /// Identifies exactly which pages a subject read, it goes into the session marker.
    pub hash: String,
}

/// The folder everything that belongs to a study lives in.
pub fn study_root(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_local_data_dir()?.join("study"))
}

/// Writes the default pages if the study has no instructions yet: the folder is missing or
/// empty. A folder with anything in it is never touched, a broken one is reported when the
/// study starts instead of being replaced behind the researcher's back.
///
/// Returns whether the defaults were written.
pub fn ensure_default(root: &Path) -> io::Result<bool> {
    let dir = root.join(INSTRUCTIONS_DIR);
    if dir.exists() && fs::read_dir(&dir)?.next().is_some() {
        return Ok(false);
    }
    for (lang, markdown) in DEFAULT_PAGES {
        let lang_dir = dir.join(lang);
        fs::create_dir_all(&lang_dir)?;
        fs::write(lang_dir.join(DEFAULT_FILE), markdown)?;
    }
    info!("Wrote default instructions to {}", dir.display());
    Ok(true)
}

/// Splits `NN_name.md` into its number. `None` for anything that does not follow the pattern.
fn page_number(file: &str) -> Option<u32> {
    let stem = file
        .strip_suffix(".md")
        .or_else(|| file.strip_suffix(".MD"))?;
    let (number, name) = stem.split_once('_')?;
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) || name.is_empty() {
        return None;
    }
    number.parse().ok()
}

/// Reads and validates the pages of one language. Every error is phrased for the
/// experimenter, it is shown as it is in place of the start button.
pub fn load(root: &Path, lang: &str) -> Result<InstructionSet, String> {
    // The language comes from the frontend and becomes part of a path.
    if lang.is_empty() || !lang.bytes().all(|b| b.is_ascii_lowercase()) {
        return Err(format!("\"{lang}\" is not a valid language code."));
    }
    let dir = root.join(INSTRUCTIONS_DIR).join(lang);
    let shown = dir.display().to_string();
    if !dir.is_dir() {
        return Err(format!(
            "There are no instructions for the language \"{lang}\". Create the folder {shown} \
             with at least one page named like 01_welcome.md."
        ));
    }

    let entries = fs::read_dir(&dir).map_err(|e| format!("Could not read {shown}: {e}"))?;
    let mut pages = Vec::new();
    let mut warnings = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Could not read {shown}: {e}"))?;
        let file = entry.file_name().to_string_lossy().into_owned();
        // Editors and file managers leave hidden files behind, they are nobody's typo.
        if file.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let number = match page_number(&file) {
            Some(number) if path.is_file() => number,
            _ => {
                warnings.push(format!(
                    "Skipped \"{file}\", pages have to be named like 01_welcome.md."
                ));
                continue;
            }
        };
        let markdown = fs::read_to_string(&path)
            .map_err(|e| format!("Could not read the page \"{file}\": {e}"))?;
        pages.push(InstructionPage {
            file,
            number,
            markdown,
        });
    }

    if pages.is_empty() {
        return Err(format!(
            "The folder {shown} has no instruction pages. Add at least one page named like \
             01_welcome.md."
        ));
    }
    // The file name breaks ties only to keep the order stable until the error below fires.
    pages.sort_by(|a, b| a.number.cmp(&b.number).then_with(|| a.file.cmp(&b.file)));
    if let Some(pair) = pages.windows(2).find(|pair| pair[0].number == pair[1].number) {
        return Err(format!(
            "The pages \"{}\" and \"{}\" have the same number, the order between them is \
             unclear.",
            pair[0].file, pair[1].file
        ));
    }
    warnings.sort();
    for warning in &warnings {
        warn!("{warning}");
    }

    Ok(InstructionSet {
        lang: lang.to_owned(),
        dir: shown,
        hash: hash_pages(&pages),
        pages,
        warnings,
    })
}

/// A digest over the file names and contents, in page order. Renaming a page changes it too,
/// since the name is all a researcher has to tell two versions apart.
fn hash_pages(pages: &[InstructionPage]) -> String {
    let mut hasher = Sha256::new();
    for page in pages {
        hasher.update(page.file.as_bytes());
        hasher.update([0]);
        hasher.update(page.markdown.as_bytes());
        hasher.update([0]);
    }
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The instruction pages of `lang`. Runs right before every study start, so edits show up
/// without restarting the app. A study without instructions gets the defaults first, in case
/// the folder was removed while the app was running.
#[tauri::command]
pub fn load_instructions(lang: String, app: AppHandle) -> Result<InstructionSet, String> {
    let root = study_root(&app).map_err(|e| format!("Could not find the study folder: {e}"))?;
    ensure_default(&root).map_err(|e| format!("Could not create the default instructions: {e}"))?;
    load(&root, &lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, lang: &str, file: &str, content: &str) {
        let dir = root.join(INSTRUCTIONS_DIR).join(lang);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(file), content).unwrap();
    }

    fn files(set: &InstructionSet) -> Vec<&str> {
        set.pages.iter().map(|page| page.file.as_str()).collect()
    }

    #[test]
    fn page_names() {
        assert_eq!(page_number("01_welcome.md"), Some(1));
        assert_eq!(page_number("12_two_words.md"), Some(12));
        assert_eq!(page_number("1_short.md"), Some(1));
        assert_eq!(page_number("01_.md"), None);
        assert_eq!(page_number("_welcome.md"), None);
        assert_eq!(page_number("ab_welcome.md"), None);
        assert_eq!(page_number("01-welcome.md"), None);
        assert_eq!(page_number("01_welcome.txt"), None);
        assert_eq!(page_number("01_welcome.md.bak"), None);
    }

    #[test]
    fn defaults_are_written_into_a_missing_folder() {
        let root = tempfile::tempdir().unwrap();
        assert!(ensure_default(root.path()).unwrap());
        for (lang, markdown) in DEFAULT_PAGES {
            let set = load(root.path(), lang).unwrap();
            assert_eq!(files(&set), [DEFAULT_FILE]);
            assert_eq!(set.pages[0].markdown, *markdown);
        }
    }

    #[test]
    fn defaults_are_written_into_an_empty_folder() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(INSTRUCTIONS_DIR)).unwrap();
        assert!(ensure_default(root.path()).unwrap());
    }

    /// A researcher's folder is never overwritten, not even when it is broken.
    #[test]
    fn existing_instructions_are_left_alone() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "en", "notes.txt", "");
        assert!(!ensure_default(root.path()).unwrap());
        assert!(!root.path().join(INSTRUCTIONS_DIR).join("de").exists());
        assert!(load(root.path(), "en").is_err());
    }

    #[test]
    fn pages_are_ordered_by_number_not_by_name() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "en", "10_last.md", "c");
        write(root.path(), "en", "02_middle.md", "b");
        write(root.path(), "en", "1_first.md", "a");
        let set = load(root.path(), "en").unwrap();
        assert_eq!(files(&set), ["1_first.md", "02_middle.md", "10_last.md"]);
        assert!(set.warnings.is_empty());
    }

    #[test]
    fn stray_files_are_skipped_with_a_warning() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "en", "01_welcome.md", "a");
        write(root.path(), "en", "welcome.md", "b");
        write(root.path(), "en", ".01_welcome.md.swp", "c");
        fs::create_dir_all(root.path().join(INSTRUCTIONS_DIR).join("en").join("02_dir.md"))
            .unwrap();
        let set = load(root.path(), "en").unwrap();
        assert_eq!(files(&set), ["01_welcome.md"]);
        assert_eq!(set.warnings.len(), 2);
    }

    #[test]
    fn a_duplicate_number_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "en", "01_welcome.md", "a");
        write(root.path(), "en", "1_intro.md", "b");
        let error = load(root.path(), "en").unwrap_err();
        assert!(error.contains("01_welcome.md") && error.contains("1_intro.md"));
    }

    #[test]
    fn a_missing_language_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "de", "01_willkommen.md", "a");
        assert!(load(root.path(), "en").is_err());
    }

    #[test]
    fn a_language_without_pages_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "en", "readme.txt", "a");
        assert!(load(root.path(), "en").is_err());
    }

    #[test]
    fn the_language_cannot_leave_the_instructions_folder() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "en", "01_welcome.md", "a");
        assert!(load(root.path(), "../instructions/en").is_err());
        assert!(load(root.path(), "").is_err());
    }

    #[test]
    fn the_hash_follows_names_and_contents() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "en", "01_welcome.md", "a");
        let first = load(root.path(), "en").unwrap().hash;
        assert_eq!(load(root.path(), "en").unwrap().hash, first);

        write(root.path(), "en", "01_welcome.md", "b");
        let edited = load(root.path(), "en").unwrap().hash;
        assert_ne!(edited, first);

        fs::rename(
            root.path().join(INSTRUCTIONS_DIR).join("en").join("01_welcome.md"),
            root.path().join(INSTRUCTIONS_DIR).join("en").join("01_hello.md"),
        )
        .unwrap();
        assert_ne!(load(root.path(), "en").unwrap().hash, edited);
    }
}
