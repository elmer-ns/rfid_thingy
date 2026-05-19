#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(never_type)]

use alloc::vec::Vec;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Instant, Timer};
use esp_hal::{Blocking, peripherals::GPIO38, rmt::Rmt};
use esp_hal_smartled::smart_led_buffer;
use mfrc522::MifareKey;
use serde::{Deserialize, Serialize, ser::SerializeStruct};
use serde_big_array::BigArray;
use smart_leds::{SmartLedsWrite, brightness};

use crate::rfid::{BLOCK_USIZE, CARD_USIZE, SAFE_SECTOR_USIZE, SECTOR_USIZE};

pub mod helpers;
pub mod rfid;
pub mod web;
pub mod wifi;

extern crate alloc;

#[embassy_executor::task]
pub async fn light_task(rmt: Rmt<'static, Blocking>, gpio: GPIO38<'static>) -> ! {
    let mut led_buffer = smart_led_buffer!(1);

    let mut onboard_led =
        { esp_hal_smartled::SmartLedsAdapter::new(rmt.channel0, gpio, &mut led_buffer) };

    #[derive(PartialEq, Eq)]
    enum State {
        Uninit,
        Active,
        Inactive,
    }

    let mut state = State::Uninit;

    loop {
        Timer::after(Duration::from_millis(500)).await;

        let active = STATE
            .lock(|state: &mut crate::State| state.reader_active)
            .await;

        let new_state = {
            if active {
                State::Active
            } else {
                State::Inactive
            }
        };

        if new_state == state {
            continue;
        } else {
            state = new_state;
        }

        let colors = match &state {
            State::Active => [smart_leds::colors::GREEN].into_iter(),
            State::Inactive => [smart_leds::colors::RED].into_iter(),
            State::Uninit => unreachable!(),
        };

        onboard_led.write(brightness(colors, 10)).unwrap();
    }
}

#[macro_export]
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

/// Stores main reader state which is used across tasks
pub static STATE: StateMutex = StateMutex(Mutex::new(State {
    reader_active: false,
    reader_operation: ReaderOperation::None,
    history: Vec::new(),
}));

/// Safe abstraction over raw [Mutex] containing a [RefCell]. Stores main reader state which is used across tasks
pub struct StateMutex(Mutex<CriticalSectionRawMutex, State>);

impl StateMutex {
    /// Lock the mutex, borrowing its contents for the duration of a closure
    pub async fn lock<F: FnOnce(&mut State) -> T, T>(&self, f: F) -> T {
        let mut value = self.0.lock().await;
        f(&mut value)
    }
}

#[derive(Serialize, Clone)]
/// Stores main reader state which is used across tasks
pub struct State {
    pub reader_active: bool,
    pub reader_operation: ReaderOperation,
    pub history: Vec<HistoryItem>,
}

#[derive(Clone)]
/// Stores a [ReaderEvent] and the point in time at which it occured
pub struct HistoryItem {
    pub event: ReaderEvent,
    pub timestamp: Instant,
}

impl Serialize for HistoryItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("HistoryItem", 2)?;
        state.serialize_field("event", &self.event)?;
        state.serialize_field("timestamp", &self.timestamp.as_millis())?;
        state.end()
    }
}

#[derive(Clone, Serialize, Deserialize)]
/// Defines what different operations a reader can do when it detects a card
pub enum ReaderOperation {
    /// Do nothing (other than recording the interaction as a [ReaderEvent::Found])
    None,
    /// Read the data from a specific block on the detected card
    ReadBlock { block: u8, key: MifareKey },
    /// Read the data from a specific sector on the detected card
    ReadSector { sector: u8, key: MifareKey },
    /// Read all the data from the detected card
    ReadCard { keys: [MifareKey; CARD_USIZE] },
    /// Write some data to a specific block on the detected card
    WriteBlock {
        block: u8,
        key: MifareKey,
        data: [u8; BLOCK_USIZE],
    },
    /// Write some data to a specific sector on the detected card (can only write to the 3 blocks in the sectors, for safety reasons)
    WriteSector {
        sector: u8,
        key: MifareKey,
        #[serde(with = "BigArray")]
        data: [u8; BLOCK_USIZE * SAFE_SECTOR_USIZE],
    },
    /// Write some data to the detected card (can only write to the 3 first blocks in a sector, for safety reasons)
    WriteCard {
        key: [MifareKey; CARD_USIZE],
        #[serde(with = "BigArray")]
        data: [u8; BLOCK_USIZE * SAFE_SECTOR_USIZE * CARD_USIZE],
    },
}

impl From<mfrc522::Uid> for Uid {
    fn from(value: mfrc522::Uid) -> Self {
        Uid(value.as_bytes().to_vec())
    }
}

impl From<&mfrc522::Uid> for Uid {
    fn from(value: &mfrc522::Uid) -> Self {
        Uid(value.as_bytes().to_vec())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Uid(Vec<u8>);

#[derive(Debug, Clone, Serialize)]
/// Event recorded by the main reader task
pub enum ReaderEvent {
    /// Setup is complete, and the main reader task has begun
    Boot,
    Found {
        uid: Uid,
    },
    ReadBlock {
        uid: Uid,
        block: u8,
        data: [u8; BLOCK_USIZE],
    },
    ReadSector {
        uid: Uid,
        sector: u8,
        #[serde(with = "BigArray")]
        data: [u8; BLOCK_USIZE * SECTOR_USIZE],
    },
    ReadCard {
        uid: Uid,
        #[serde(with = "BigArray")]
        data: [u8; BLOCK_USIZE * SECTOR_USIZE * CARD_USIZE],
    },
    WroteBlock {
        uid: Uid,
        block: u8,
    },
    WroteSector {
        uid: Uid,
        sector: u8,
    },
    WroteCard {
        uid: Uid,
    },
}
