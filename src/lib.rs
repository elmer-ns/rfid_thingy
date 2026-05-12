#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(never_type)]

use alloc::vec::Vec;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_time::{Duration, Instant, Timer};
use esp_hal::{Blocking, peripherals::GPIO38, rmt::Rmt, time::Rate};
use esp_hal_smartled::{SmartLedsAdapter, smart_led_buffer};
use mfrc522::MifareKey;
use serde::Serialize;
use serde_big_array::BigArray;
use smart_leds::{SmartLedsWrite, brightness};

use crate::rfid::{BLOCK_USIZE, CARD_USIZE, SECTOR_USIZE};

pub mod helpers;
pub mod rfid;
pub mod web;
pub mod wifi;

extern crate alloc;

#[embassy_executor::task]
pub async fn light_task(rmt: Rmt<'static, Blocking>, gpio: GPIO38<'static>) -> ! {
    let mut led_buffer = smart_led_buffer!(1);

    let mut onboard_led = {
        let frequency = Rate::from_mhz(80);
        //let rmt = Rmt::new(peripherals.RMT, frequency).expect("Failed to initialize RMT0");
        esp_hal_smartled::SmartLedsAdapter::new(rmt.channel0, gpio, &mut led_buffer)
    };

    enum State {
        Active,
        Inactive,
        Detected { at: embassy_time::Instant },
    }

    let mut state = State::Inactive;

    loop {
        let active = STATE.lock(|state| state.reader_active);

        match &mut state {
            State::Active => {
                if !active {
                    state = State::Inactive
                }
            }
            State::Inactive => {
                if active {
                    state = State::Active
                }
            }
            State::Detected { at } => {
                const ACTIVE_FOR: Duration = Duration::from_millis(500);
                if Instant::now().duration_since(*at) > ACTIVE_FOR {
                    if active {
                        state = State::Active
                    } else {
                        state = State::Inactive
                    }
                }
            }
        }

        let colors = match &state {
            State::Active => [smart_leds::colors::GREEN].into_iter(),
            State::Inactive => [smart_leds::colors::RED].into_iter(),
            State::Detected { at: _ } => [smart_leds::colors::BLUE].into_iter(),
        };

        onboard_led.write(brightness(colors, 10)).unwrap();

        Timer::after(Duration::from_millis(500)).await;
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

pub static STATE: Mutex<CriticalSectionRawMutex, State> = Mutex::new(State {
    reader_active: false,
    reader_operation: ReaderOperation::None,
});

#[derive(Serialize, Clone)]
pub struct State {
    pub reader_active: bool,
    pub reader_operation: ReaderOperation,
}

#[derive(Clone, Serialize)]
pub enum ReaderOperation {
    None,
    ReadBlock {
        block: u8,
        key: MifareKey,
    },
    ReadSector {
        sector: u8,
        key: MifareKey,
    },
    ReadCard {
        keys: [MifareKey; CARD_USIZE],
    },
    WriteBlock {
        block: u8,
        key: MifareKey,
        data: [u8; BLOCK_USIZE],
    },
    WriteSector {
        sector: u8,
        key: MifareKey,
        #[serde(with = "BigArray")]
        data: [u8; BLOCK_USIZE * SECTOR_USIZE],
    },
    WriteCard {
        key: [MifareKey; CARD_USIZE],
        #[serde(with = "BigArray")]
        data: [u8; BLOCK_USIZE * SECTOR_USIZE * CARD_USIZE],
    },
}

enum DataWithMeta {
    Block {
        data: [u8; BLOCK_USIZE],
        block: u8,
        key: MifareKey,
    },
    Sector {
        data: [u8; BLOCK_USIZE * SECTOR_USIZE],
        sector: u8,
        key: MifareKey,
    },
    Card {
        data: [u8; BLOCK_USIZE * SECTOR_USIZE * CARD_USIZE],
        key: [MifareKey; CARD_USIZE],
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
pub enum ReaderInteraction {
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
}
