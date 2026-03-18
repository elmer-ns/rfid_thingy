#![no_std]

use core::fmt::Debug;

use alloc::vec::Vec;
use embassy_time::{Duration, Timer};
use mfrc522::{AtqA, Initialized, Mfrc522, Uid, comm::Interface};

extern crate alloc;

pub struct Reader<E: Debug, COMM: Interface<Error = E>> {
    reader: Mfrc522<COMM, Initialized>,    
}

pub struct CardInteraction<'r, E: Debug, COMM: Interface<Error = E>> {
    reader: &'r mut Mfrc522<COMM, Initialized>,

    atqa: AtqA,
    uid: Option<Uid>,
    auth_section: Option<u8>,
}

impl<'r, E: Debug, COMM: Interface<Error = E>> CardInteraction<'r, E, COMM> {}

type Sector = [Block; SECTOR_SIZE];
type Block = [u8; BLOCK_SIZE];

impl<E: Debug, COMM: Interface<Error = E>> Reader<E, COMM> {
    pub fn new(comm: COMM) -> Option<Self> {
        let reader = Mfrc522::new(comm).init().ok()?;

        Some(Self { reader })
    }

    pub async fn wait_for_card<'r>(&'r mut self) -> Result<CardInteraction<'r, E, COMM>, mfrc522::Error<E>> {
        loop {
            match self.reader.new_card_present() {
                Err(mfrc522::Error::Timeout) => Timer::after(Duration::from_millis(25)).await,
                result => {
                    return Ok(CardInteraction { reader: &mut self.reader, atqa: result?, uid: None, auth_section: None });
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