# footprint

A desktop app for building and running study protocols with emotional images. You put a session
together from blocks of trials: subjects see an image, optionally perform a paced walking task,
and rate what they saw. The app handles timing, randomisation and data collection, and publishes
an event marker for every phase over [Lab Streaming Layer (LSL)](https://labstreaminglayer.org).
That lets the trials be aligned with EEG, motion capture, physiological sensors or any other
recording on the same LSL network.

Walking is one building block, not a requirement: a protocol can consist entirely of image
presentation and ratings.

It is built with [Tauri 2](https://tauri.app) (Rust backend) and SvelteKit (frontend), and runs
on Windows, macOS and Linux.

## How a study protocol works

A protocol is a list of **blocks**, set up on the experiment page. Each block has a kind, and the
kind decides what happens in it and which settings it has. Blocks of different kinds can be mixed
freely in one session.

| Block kind | What happens |
|---|---|
| `static_stimulus` | Trials: fixation cross → image → rating |
| `stimulus` | Trials: fixation cross → image → paced walk → rating |
| `neutral` | Trials: fixation cross → fixation cross in place of the image → paced walk → "press Enter" |
| `pause` | No trials, one break screen until the experimenter presses Enter |

### `static_stimulus`

Image presentation without movement. Each trial shows a fixation cross, then an image, then the
rating screen. The subject rates the image from 1 to 7 on the number keys and confirms with
Enter, once per enabled scale.

Settings: number of trials, rating scales (valence, arousal, both or none), and how long the
fixation cross and the image are shown, each with random jitter.

### `stimulus`

Like `static_stimulus`, with a paced walk between the image and the rating. After the image, a
start cue plays: a sound and/or a green flash, depending on the settings. A triangle then moves
along a track on screen at the target speed. The subject walks with it from the middle to one
end, turns around within 3 seconds, walks to the other end, turns again and walks back to the
middle.

Settings: everything `static_stimulus` has, plus the speed variations (very slow, slow, fast,
very fast) added on top of the subject's normal speed. The speed conditions are balanced within
the block.

### `neutral`

A walking control condition without emotional content. Each trial has the same timing and walk
as a `stimulus` trial, but a fixation cross stays in place of the image. Since there is nothing
to rate, the trial ends with a "press Enter to continue" screen.

Settings: number of trials, the speed variations, and the durations of both fixation-cross phases
(the baseline and the one in place of the image).

### `pause`

A break between blocks. It has no trials, shows a single screen, and waits for the experimenter
to press Enter. It has no settings.

### Images

Images are drawn without replacement from four quadrants (low/high valence × low/high arousal)
across the whole session, so no image appears twice, not even across blocks.

### Before the first session

Open the settings to:

- Choose the **language** shown to the subject (`de` or `en`) and the start cues.
- For walking blocks, **calibrate the walking speed**: the subject walks a known distance a few
  times and the app averages their speed. That becomes the subject's "normal" speed, and the
  speed variations are derived from it.
- For walking blocks, set the **walking distance per trial**: the full middle → end → end →
  middle path, in metres.

> Currently, Start requires a walking speed and distance even when no block walks. Enter any
> value in that case.

### Output

- **CSV:** one row per trial (and per pause), with the timestamps of every phase, the speed
  condition, the image and the ratings. It is written to `~/footprints/<study name>/<subject>_<timestamp>.csv`
  when the run ends.
- **LSL:** a string marker stream named `App Events` (type `Markers`) with one JSON object per
  event. The format is documented in [docs/marker-protocol.md](docs/marker-protocol.md).

A **practice run** (press `T` on any instruction page) runs three trials that use the rating scales
of the protocol. Its markers are tagged `test`, so it cannot be confused with real data.

> Currently the practice trials always include the walking phase, even when no block in the
> protocol walks.

### Stimulus images

The bundled images are from the **Open Affective Standardized Image Set (OASIS)**, sorted into
the four quadrants by their normative valence and arousal ratings. If you use them, cite:

> Kurdi, B., Lozano, S., & Banaji, M. R. (2017). Introducing the Open Affective Standardized
> Image Set (OASIS). *Behavior Research Methods, 49*(2), 457–470.
> https://doi.org/10.3758/s13428-016-0715-3

Before publishing or redistributing the images, check the OASIS usage terms.

## Instructions

The pages a subject reads before the first trial belong to the study, not to the app. You write
them as Markdown files, one file per page:

```
<study folder>/instructions/
├── de/
│   ├── 01_welcome.md
│   ├── 02_rating.md
│   └── 03_procedure.md
└── en/
    ├── 01_welcome.md
    ├── 02_rating.md
    └── 03_procedure.md
```

The study folder is `study/` inside the app's local data directory:

| OS | Study folder |
|---|---|
| Linux | `~/.local/share/com.footprint.app/study/` |
| Windows | `%LOCALAPPDATA%\com.footprint.app\study\` |
| macOS | `~/Library/Application Support/com.footprint.app/study/` |

The experiment page shows the exact folder it read the pages from.

### Rules

- One folder per language, named after the language code the app uses (`de`, `en`). The pages
  of the language selected in the settings are shown.
- Pages are named `NN_name.md` and shown in the order of their number. Gaps are fine, so you can
  insert `05_breaks.md` between `01_…` and `10_…` without renaming anything.
- Two pages with the same number (`01_a.md` and `1_b.md`) is an error, because their order would
  be unclear.
- Files that do not follow the pattern are skipped, and the experiment page lists them as
  warnings. Hidden files (starting with `.`) are ignored.
- Pages are standard Markdown: headings, bold, lists, quotes and so on. Embedded HTML is
  sanitized and scripts never run. Images inside pages are not supported yet.
- The app adds the navigation itself: **Enter** goes to the next page (and starts the study on
  the last one), **Backspace** goes back, **T** opens a practice run. Each page gets a footer
  with these hints and a page counter, so you do not need to explain the keys in your pages.

### Defaults and errors

If the `instructions/` folder is missing or empty, the app creates a single default page per
language (`01_default.md`, "press Enter to start the study") on startup. Replace it with your
own pages. A folder that already has anything in it is never touched.

The pages are checked when the experiment page opens and read again when you press **Start**, so
edits take effect without restarting the app. If the folder for the selected language is
missing, has no pages, or has two pages with the same number, Start stays disabled and the page
tells you what to fix.

### Which pages a subject read

The `session` marker at the start of every run records the language, the page files in order,
and a hash of their names and contents. If you change the instructions during a study, the hash
tells you which sessions saw which version.

## Development

Prerequisites:

- Rust ≥ 1.82
- [bun](https://bun.sh) 1.2.24 (lower versions might work too)
- CMake and a C++ compiler, to build liblsl
- On Linux, the Tauri system packages:
  `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libasound2-dev`

```sh
bun i
bun tauri dev      # run the app with hot reload
bun tauri build    # build an installer for the current platform
```

Checks:

```sh
bun run check                          # Svelte and TypeScript
cargo test --manifest-path src-tauri/Cargo.toml
```

### Layout

| Path | What lives there |
|---|---|
| `src/routes/experiment/` | Experiment page, block editor and the runner that drives a session |
| `src/routes/calibration/` | Speed calibration window |
| `src/components/experiment/` | One screen per trial phase (instructions, baseline, walk, rating, …) |
| `src/lib/state_machine.ts` | The trial state machine; every phase change publishes a marker here |
| `src/lib/blocks_state.ts` | Blocks and how they turn into a trial plan |
| `src-tauri/src/` | Backend: LSL stream, CSV logger, image pool, instruction loading |
| `static/images/` | Stimulus images, one folder per valence/arousal quadrant |
| `docs/` | The LSL marker protocol |

## Contributing

- Branch off `main` and open a pull request against it. Pushing to `release` builds and publishes
  the installers, so only merge there for a release.
- Write commit messages in the [Conventional Commits](https://www.conventionalcommits.org) style
  used so far: `feat: …`, `fix: …`, `chore: …`, `ci: …`.
- Run `bun run check` and `cargo test` before opening a PR.
- The LSL markers and the CSV columns are what researchers analyse, so treat them as a public
  interface. If you change either, update [docs/marker-protocol.md](docs/marker-protocol.md) and
  the tests in `src-tauri/src/lsl.rs` and `src-tauri/src/logger.rs`, and say so in the PR.
- A new block kind gets a row in the block table and its own section under
  [How a study protocol works](#how-a-study-protocol-works), plus its markers in the protocol
  doc.
- Experimenter-facing UI is in English. Subject-facing text goes through `src/lib/lang/*.json`
  or, for instructions, the study's Markdown files.
