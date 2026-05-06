#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(never_type)]

use alloc::vec::Vec;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use mfrc522::MifareKey;
use serde::Serialize;
use serde_big_array::BigArray;

use crate::rfid::{BLOCK_USIZE, CARD_USIZE, SECTOR_USIZE};

pub mod rfid;
pub mod web;
pub mod wifi;

extern crate alloc;

#[macro_export]
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

static STATE: Mutex<CriticalSectionRawMutex, State> = Mutex::new(State {
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
    Read {
        block: u8,
        read_sector: bool,
        key: MifareKey,
    },
    Write {
        pos: u8,
        data: CardData,
        key: MifareKey,
    },
}

enum DataWithMeta {
    Block {
        data: [u8; BLOCK_USIZE],
        block: u8,
        key: MifareKey,    
    },
    Sector {
        data: [u8; BLOCK_USIZE*SECTOR_USIZE],
        sector: u8,
        key: MifareKey,
    },
    Card {
        data: [u8; BLOCK_USIZE*SECTOR_USIZE*CARD_USIZE],
        key: [MifareKey; CARD_USIZE],
    }
}

impl From<mfrc522::Uid> for Uid {
    fn from(value: mfrc522::Uid) -> Self {
        Uid(value.as_bytes().to_vec())
    }
}

#[derive(Debug, Clone, Serialize)]
struct Uid(Vec<u8>);

#[derive(Debug, Copy, Clone, Serialize)]
enum CardData {
    #[serde(with = "BigArray")]
    Block([u8; BLOCK_USIZE]),
    #[serde(with = "BigArray")]
    Sector([u8; BLOCK_USIZE*SECTOR_USIZE]),
    #[serde(with = "BigArray")]
    Card([u8; BLOCK_USIZE*SECTOR_USIZE*CARD_USIZE]),
}

#[derive(Clone, Serialize)]
pub enum ReaderInteraction {
    Found {
        uid: Uid,
    },
    Read {
        uid: Uid,
        block: u8,
        data: CardData,
    },
    Write {
        uid: Uid,
        block: u8,
        data: CardData,
    },
}
