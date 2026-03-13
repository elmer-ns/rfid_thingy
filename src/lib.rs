#![no_std]

use core::fmt::Debug;

use mfrc522::{Initialized, Mfrc522, Uid, comm::Interface};

pub struct Reader<E: Debug, COMM: Interface<Error = E>> {
    reader: Mfrc522<COMM, Initialized>,
}

type Block = [u8;16];
type Key = [u8; 6];

impl<E: Debug, COMM: Interface<Error = E>> Reader<E, COMM> {
    pub fn new(comm: COMM) -> Option<Self> {
        let reader = Mfrc522::new(comm).init().ok()?;

        Some(Self { reader })
    }

    fn read_block(&mut self, uid: &Uid, block: u8, key: &Key) -> Block {
        let b = [];

        self.handle_auth(uid, block, key, |reader| {
            let read = reader.mf_read(block)?;
        });
    }

    fn handle_auth<F: FnOnce(&mut Mfrc522<COMM, Initialized>) -> Result<(), mfrc522::Error<E>>>(&mut self, uid: &Uid, block: u8, key: &Key, action: F) -> Result<(), mfrc522::Error<E>> {
        self.reader.mf_authenticate(uid, block, key)?;

        action(&mut self.reader)?;

        self.reader.hlta()?;

        Ok(())
    }

}