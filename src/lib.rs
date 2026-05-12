#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(never_type)]

use alloc::vec::Vec;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use mfrc522::MifareKey;
use serde::Serialize;
use serde_big_array::BigArray;

use crate::rfid::{BLOCK_USIZE, CARD_USIZE, SECTOR_USIZE};

pub mod helpers;
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
