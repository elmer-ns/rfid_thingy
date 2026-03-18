#![no_std]

use core::{array, fmt::Debug};

use alloc::vec::Vec;
use embassy_time::{Duration, Timer};
use log::info;
use mfrc522::{AtqA, Initialized, Mfrc522, Uid, comm::Interface};

extern crate alloc;

pub struct Reader<E: Debug, COMM: Interface<Error = E>> {
    reader: Mfrc522<COMM, Initialized>,

    atqa: Option<AtqA>,
    uid: Option<Uid>,
    auth_sector: Option<u8>,
    
}

type Sector = [Block; SECTOR_SIZE];
type Block = [u8; BLOCK_SIZE];

impl<E: Debug, COMM: Interface<Error = E>> Reader<E, COMM> {
    pub fn new(comm: COMM) -> Option<Self> {
        let reader = Mfrc522::new(comm).init().ok()?;

        Some(Self { reader, atqa: None, uid: None, auth_sector: None })
    }

    pub async fn wait_for_card(&mut self) -> Result<(), mfrc522::Error<E>> {
        loop {
            match self.reader.new_card_present() {
                Err(mfrc522::Error::Timeout) => Timer::after(Duration::from_millis(25)).await,
                result => {
                    self.atqa = Some(result?);
                    return Ok(());
                }
            }
        }
    }
}

const N_SECTORS: usize = 16;
const BLOCK_SIZE: usize = 16;
const SECTOR_SIZE: usize = 4;

type Tag = Vec<Sector>;

type SectorKey = [u8; 6];
type TagKey = Vec<SectorKey>;