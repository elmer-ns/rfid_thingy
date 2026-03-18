#![no_std]

use core::{array, fmt::Debug};

use alloc::vec::Vec;
use embassy_time::{Duration, Timer};
use log::info;
use mfrc522::{AtqA, Initialized, Mfrc522, Uid, comm::Interface};

extern crate alloc;

pub struct Reader<E: Debug, COMM: Interface<Error = E>> {
    pub reader: Mfrc522<COMM, Initialized>,
}

type Sector = [Block; SECTOR_SIZE];
type Block = [u8; BLOCK_SIZE];

impl<E: Debug, COMM: Interface<Error = E>> Reader<E, COMM> {
    pub fn new(comm: COMM) -> Option<Self> {
        let reader = Mfrc522::new(comm).init().ok()?;

        Some(Self { reader })
    }

    pub fn select(&mut self, atqa: &AtqA) -> Result<Uid, mfrc522::Error<E>> {
        self.reader.select(atqa)
    }

    pub async fn new_card_present_async(&mut self) -> Result<AtqA, mfrc522::Error<E>> {
        loop {
            match self.new_card_present() {
                Err(mfrc522::Error::Timeout) => Timer::after(Duration::from_millis(20)).await,
                result => return result
            }
        }
    }

    pub fn new_card_present(&mut self) -> Result<AtqA, mfrc522::Error<E>> {
        self.reader.new_card_present()
    }

    pub fn read_block(&mut self, uid: &Uid, block: u8, key: &SectorKey) -> Result<Block, mfrc522::Error<E>> {
        let mut b = [0; 16];

        self.handle_auth(uid, block, key, |reader| {
            let read = reader.mf_read(block)?;

            b = read;

            Ok(())
        })?;

        //self.reader.hlta()?;

        Ok(b)
    }

    pub fn read_sector(&mut self, uid: &Uid, sector: u8, key: &SectorKey) -> Result<Sector, mfrc522::Error<E>> {
        let mut s: Sector = array::from_fn(|_| [0; 16]);

        let block = sector * 4;

        self.handle_auth(uid, block, key, |reader| {
            let b0 = reader.mf_read(block)?;
            let b1 = reader.mf_read(block+1)?;
            let b2 = reader.mf_read(block+2)?;
            let b3 = reader.mf_read(block+3)?;

            s = [b0, b1, b2, b3];

            Ok(())
        })?;

        //self.reader.hlta()?;

        Ok(s)
    }

    pub fn read_tag(&mut self, uid: &Uid, key: &TagKey) -> Result<Tag, mfrc522::Error<E>> {
        let mut tag = Vec::new();

        for (sector, sector_key) in key.iter().enumerate() {
            let block = (sector * 4) as u8;
            info!("sector {}", sector);
            self.handle_auth(uid, block, sector_key, |reader| {
                tag.push([reader.mf_read(block).unwrap(), reader.mf_read(block+1).unwrap(), reader.mf_read(block+2).unwrap(), reader.mf_read(block+3).unwrap()]);

                Ok(())
            }).unwrap();
        }

        //self.reader.hlta()?;

        Ok(tag)
    }

    fn handle_auth<T, F: FnOnce(&mut Mfrc522<COMM, Initialized>) -> Result<T, mfrc522::Error<E>>>(&mut self, uid: &Uid, block: u8, key: &SectorKey, action: F) -> Result<T, mfrc522::Error<E>> {
        self.reader.mf_authenticate(uid, block, key)?;

        let out = action(&mut self.reader);

        self.reader.stop_crypto1()?;

        Ok(out?)
    }

}

const N_SECTORS: usize = 16;
const BLOCK_SIZE: usize = 16;
const SECTOR_SIZE: usize = 4;

type Tag = Vec<Sector>;

type SectorKey = [u8; 6];
type TagKey = Vec<SectorKey>;