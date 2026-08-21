use log::info;
use lsl::{Pushable, StreamOutlet};
use serde::Deserialize;
use std::{
    sync::{
        atomic::{AtomicI32, Ordering},
        mpsc::{channel, Receiver, Sender},
    },
    thread::{sleep, spawn},
    time::Duration,
};

pub struct LsLManager {
    sender: Sender<LsLEvent>,
}

pub struct LsL {
    state: AtomicI32,
    recv: Receiver<LsLEvent>,
    event_outlet: StreamOutlet,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Deserialize)]
pub enum LsLEvent {
    Idle = -1,
    Baseline = 0,
    Stimulus = 1,
    Movement = 2,
    Rating = 3,
}

impl LsL {
    fn new(rx: Receiver<LsLEvent>) -> Result<Self, lsl::Error> {
        let info = lsl::StreamInfo::new(
            "App Events",
            "Markers",
            1,
            lsl::IRREGULAR_RATE,
            lsl::ChannelFormat::Int64,
            "",
        )?;

        Ok(Self {
            state: AtomicI32::new(-1),
            recv: rx,
            event_outlet: StreamOutlet::new(&info, 1, 360)?,
        })
    }

    fn start(&self) {
        loop {
            if let Ok(e) = self.recv.try_recv() {
                info!("Received Event: {e:?}");
                self.state.store(e as i32, Ordering::Release);
            }
            if let Err(e) = self
                .event_outlet
                .push_sample(&[self.state.load(Ordering::Acquire)])
            {
                info!("Error sending lsl packet: {e:?}");
            }
            sleep(Duration::from_millis(10));
        }
    }
}

impl LsLManager {
    pub fn new() -> Self {
        let (tx, rx) = channel::<LsLEvent>();
        spawn(move || {
            let lsl = LsL::new(rx).unwrap();
            lsl.start();
        });
        Self { sender: tx }
    }

    pub fn publish_event(&self, event: LsLEvent) -> anyhow::Result<()> {
        self.sender.send(event)?;
        Ok(())
    }
}
