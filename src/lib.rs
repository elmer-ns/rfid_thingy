#![no_std]

use core::{array, fmt::Debug};

use mfrc522::{Initialized, Mfrc522, Uid, comm::Interface};

pub struct Reader<E: Debug, COMM: Interface<Error = E>> {
    reader: Mfrc522<COMM, Initialized>,
}

type Sector = [Block; 4];
type Block = [u8;16];
type Key = [u8; 6];

impl<E: Debug, COMM: Interface<Error = E>> Reader<E, COMM> {
    pub fn new(comm: COMM) -> Option<Self> {
        let reader = Mfrc522::new(comm).init().ok()?;

        Some(Self { reader })
    }

    pub fn read_block(&mut self, uid: &Uid, block: u8, key: &Key) -> Result<Block, mfrc522::Error<E>> {
        let mut b = [0; 16];

        self.handle_auth(uid, block, key, |reader| {
            let read = reader.mf_read(block)?;

            b = read;

            Ok(())
        })?;

        Ok(b)
    }

    pub fn read_sector(&mut self, uid: &Uid, sector: u8, key: &Key) -> Result<Sector, mfrc522::Error<E>> {
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

        Ok(s)
    }

    fn handle_auth<F: FnOnce(&mut Mfrc522<COMM, Initialized>) -> Result<(), mfrc522::Error<E>>>(&mut self, uid: &Uid, block: u8, key: &Key, action: F) -> Result<(), mfrc522::Error<E>> {
        self.reader.mf_authenticate(uid, block, key)?;

        action(&mut self.reader)?;

        self.reader.hlta()?;

        Ok(())
    }

}