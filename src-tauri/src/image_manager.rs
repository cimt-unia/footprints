use std::ffi::OsString;
use std::fs;
use std::path::Path;

use crate::rand::Alias;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{path::BaseDirectory, AppHandle, Manager};

pub struct ImageManager {
    dist: Alias<4>,
    images: Quadrants,
    test_set: Vec<Image>,
    state: State,
}

struct Quadrants {
    low_low: Vec<Image>,
    low_high: Vec<Image>,
    high_low: Vec<Image>,
    high_high: Vec<Image>,
}

struct State {
    ll: usize,
    lh: usize,
    hl: usize,
    hh: usize,
    valid_idx_ll: Vec<usize>,
    valid_idx_lh: Vec<usize>,
    valid_idx_hl: Vec<usize>,
    valid_idx_hh: Vec<usize>,
}

impl State {
    fn new(q: &Quadrants) -> Self {
        let ll = q.low_low.len();
        let lh = q.low_high.len();
        let hl = q.high_low.len();
        let hh = q.high_high.len();
        Self {
            ll,
            lh,
            hl,
            hh,
            valid_idx_ll: (0..ll).collect(),
            valid_idx_lh: (0..lh).collect(),
            valid_idx_hl: (0..hl).collect(),
            valid_idx_hh: (0..hh).collect(),
        }
    }

    fn reset(&mut self) {
        self.valid_idx_ll = (0..self.ll).collect();
        self.valid_idx_lh = (0..self.lh).collect();
        self.valid_idx_hl = (0..self.hl).collect();
        self.valid_idx_hh = (0..self.hh).collect();
    }

    fn pick(&mut self, bucket: usize) -> usize {
        let index: u64 = rand::random();
        let (idx_bucket, len) = match bucket {
            0 => (&mut self.valid_idx_ll, self.ll),
            1 => (&mut self.valid_idx_lh, self.lh),
            2 => (&mut self.valid_idx_hl, self.hl),
            3 => (&mut self.valid_idx_hh, self.hh),
            _ => unreachable!(),
        };
        // Refill idx if experiment runs for too long.
        if idx_bucket.is_empty() {
            idx_bucket.extend(0..len);
        }
        idx_bucket.swap_remove(index as usize % idx_bucket.len())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum Magnitude {
    Low,
    High,
    Test,
}

#[derive(Serialize, Clone)]
pub struct Image {
    name: String,
    valence: Magnitude,
    arousal: Magnitude,
    data: Vec<u8>,
}

/// How many images one quadrant can hold. The image id packs the quadrant into the upper 2 bits
/// of a `u16`, which leaves 14 bits for the index inside the quadrant.
const MAX_QUADRANT_SIZE: usize = 1 << 14;

/// Loads one quadrant, sorted by file name.
///
/// `read_dir` yields entries in whatever order the file system happens to keep them in. The
/// index into the returned vector is what `get_rand_image` packs into the image id and what
/// ends up in the LsL recording, so reading unsorted would hand out ids that differ between
/// machines and that cannot be resolved without the exact directory listing of that run. Only
/// `.webp` files are taken so a stray `.DS_Store` or `Thumbs.db` cannot shift every id behind
/// it. Ids still move when the image set itself changes; the trial log carries the file name
/// next to the id for that reason.
fn load_quadrant(path: &Path, valence: Magnitude, arousal: Magnitude) -> Result<Vec<Image>> {
    let mut names: Vec<OsString> = fs::read_dir(path)
        .with_context(|| format!("Failed to open image folder {}", path.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .map(|entry| entry.file_name())
        .filter(|name| {
            Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("webp"))
        })
        .collect();
    names.sort();

    ensure!(
        names.len() <= MAX_QUADRANT_SIZE,
        "{} holds {} images, an image id has room for {}",
        path.display(),
        names.len(),
        MAX_QUADRANT_SIZE
    );

    names
        .into_iter()
        .map(|name| {
            let data = fs::read(path.join(&name))
                .with_context(|| format!("Failed to read image {name:?}"))?;
            Ok(Image {
                name: name.to_string_lossy().to_string(),
                valence,
                arousal,
                data,
            })
        })
        .collect()
}

impl ImageManager {
    pub fn init(app: &AppHandle) -> Result<Self> {
        let path = app
            .path()
            .resolve("resources/images/", BaseDirectory::Resource)
            .context("Image folder not found")?;

        let q = Quadrants {
            low_low: load_quadrant(&path.join("low_low"), Magnitude::Low, Magnitude::Low)?,
            low_high: load_quadrant(&path.join("low_high"), Magnitude::Low, Magnitude::High)?,
            high_low: load_quadrant(&path.join("high_low"), Magnitude::High, Magnitude::Low)?,
            high_high: load_quadrant(&path.join("high_high"), Magnitude::High, Magnitude::High)?,
        };

        let dist = Alias::new(&[0.25; 4]);
        let state = State::new(&q);
        let test_set = std::fs::read_dir(&path.join("test"))?
            .into_iter()
            .map(|entry| {
                let entry = entry.unwrap();
                let data = fs::read(entry.path()).unwrap();
                Image {
                    name: entry.file_name().to_string_lossy().into(),
                    valence: Magnitude::Test,
                    arousal: Magnitude::Test,
                    data,
                }
            })
            .collect();

        Ok(Self {
            dist,
            images: q,
            test_set,
            state,
        })
    }

    pub fn get_test_image(&self) -> &Image {
        let idx = rand::random_range(0..self.test_set.len());
        &self.test_set[idx]
    }

    /// Refills every quadrant pool. Called once at the start of an experiment and never at a
    /// block boundary, so sampling without replacement carries on across blocks.
    pub fn reset(&mut self) {
        self.state.reset();
    }

    pub fn get_rand_image(&mut self) -> (u16, &Image) {
        let q = self.dist.generate();
        debug_assert!(q < 4);
        let i = match q {
            0 => &self.images.low_low,
            1 => &self.images.low_high,
            2 => &self.images.high_low,
            3 => &self.images.high_high,
            _ => unreachable!(),
        };
        let index = self.state.pick(q);
        // The upper 2 bits of the identifier are reserved for the quadrant, `load_quadrant`
        // caps a bucket so the index always fits in the remaining 14.
        debug_assert!(index < MAX_QUADRANT_SIZE);
        let id = (q as u16) << 14 | (index as u16);
        (id, &i[index])
    }
}
