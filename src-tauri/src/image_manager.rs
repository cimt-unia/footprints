use std::fs;

use crate::rand::Alias;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{path::BaseDirectory, AppHandle, Manager};

pub struct ImageManager {
    dist: Alias<4>,
    images: Quadrants,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Magnitude {
    Low,
    High,
}

#[derive(Serialize, Clone)]
pub struct Image {
    name: String,
    valence: Magnitude,
    arousal: Magnitude,
    data: Vec<u8>,
}

impl ImageManager {
    pub fn init(app: &AppHandle) -> Result<Self> {
        let path = app
            .path()
            .resolve("resources/images/", BaseDirectory::Resource)
            .context("Image folder not found")?;

        let mut low_low = Vec::new();
        let mut ll_p = path.clone();
        ll_p.push("low_low");
        for entry in fs::read_dir(ll_p).context("Failed to open image folder")? {
            let Ok(entry) = entry else {
                continue;
            };
            let pic = fs::read(entry.path())?;
            low_low.push(Image {
                name: entry.file_name().to_string_lossy().to_string(),
                valence: Magnitude::Low,
                arousal: Magnitude::Low,
                data: pic,
            });
        }
        let mut low_high = Vec::new();
        let mut lh_p = path.clone();
        lh_p.push("low_high");
        for entry in fs::read_dir(lh_p).context("Failed to open image folder")? {
            let Ok(entry) = entry else {
                continue;
            };
            let pic = fs::read(entry.path())?;
            low_high.push(Image {
                name: entry.file_name().to_string_lossy().to_string(),
                valence: Magnitude::Low,
                arousal: Magnitude::High,
                data: pic,
            });
        }
        let mut high_low = Vec::new();
        let mut hl_p = path.clone();
        hl_p.push("high_low");
        for entry in fs::read_dir(hl_p).context("Failed to open image folder")? {
            let Ok(entry) = entry else {
                continue;
            };
            let pic = fs::read(entry.path())?;
            high_low.push(Image {
                name: entry.file_name().to_string_lossy().to_string(),
                valence: Magnitude::High,
                arousal: Magnitude::Low,
                data: pic,
            });
        }
        let mut high_high = Vec::new();
        let mut hh_p = path.clone();
        hh_p.push("high_high");
        for entry in fs::read_dir(hh_p).context("Failed to open image folder")? {
            let Ok(entry) = entry else {
                continue;
            };
            let pic = fs::read(entry.path())?;
            high_high.push(Image {
                name: entry.file_name().to_string_lossy().to_string(),
                valence: Magnitude::High,
                arousal: Magnitude::High,
                data: pic,
            });
        }

        let dist = Alias::new(&[0.25; 4]);
        let q = Quadrants {
            low_low,
            low_high,
            high_low,
            high_high,
        };
        let state = State::new(&q);

        Ok(Self {
            dist,
            images: q,
            state,
        })
    }

    /// Refills every quadrant pool. Called once at the start of an experiment and never at a
    /// block boundary, so sampling without replacement carries on across blocks.
    pub fn reset(&mut self) {
        self.state.reset();
    }

    pub fn get_rand_image(&mut self) -> &Image {
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
        &i[index]
    }
}
