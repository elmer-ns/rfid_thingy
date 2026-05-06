use core::fmt::Debug;

use embassy_time::{Duration, Timer};
use mfrc522::{AtqA, Initialized, Mfrc522, MifareKey, Uid, comm::Interface};

extern crate alloc;

#[derive(Debug)]
pub enum Error<E> {
    ReaderError(mfrc522::Error<E>),
    NotSelected,
    OutOfBounds,

    /// You are trying to access a sector trailer (last block in each sector) mutably. Do not do that. Please stop doing that. What are you doing? DO NOT DO THAT! Please.
    SectorTrailerLock,
}

/// Wrapper around a Mfrc522 reader, with any supported interface
pub struct Reader<E: Debug, COMM: Interface<Error = E>> {
    reader: Mfrc522<COMM, Initialized>,    
}

impl<E: Debug, COMM: Interface<Error = E>> Reader<E, COMM> {
    pub fn new(comm: COMM) -> Option<Self> {
        let reader = Mfrc522::new(comm).init().ok()?;

        Some(Self { reader })
    }

    pub async fn wait_for_card<'r>(&'r mut self) -> Result<CardInteraction<'r, E, COMM>, mfrc522::Error<E>> {
        // liten delay innan vi börjar kolla efter kort. annars går saker åt helvette
        Timer::after(Duration::from_millis(50)).await;

        loop {
            match self.reader.new_card_present() {
                Err(mfrc522::Error::Timeout) => Timer::after(Duration::from_millis(50)).await,
                result => {
                    return Ok(CardInteraction { reader: &mut self.reader, atqa: result? });
                }
            }
        }
    }
}

/// Represents the interaction with a Mifare Rfid Card, and is used to interact with a card that has been detected by the reader
pub struct CardInteraction<'r, E: Debug, COMM: Interface<Error = E>> {
    reader: &'r mut Mfrc522<COMM, Initialized>,

    atqa: AtqA,
}

impl<'r, E: Debug, COMM: Interface<Error = E>> CardInteraction<'r, E, COMM> {
    /// Select this card
    pub fn select(&'r mut self) -> Result<SelectedCard<'r, E, COMM>, Error<E>> {
        let uid = self.reader.select(&self.atqa).map_err(|err| Error::ReaderError(err))?;

        Ok(SelectedCard { reader: self.reader, uid })
    }
}

/// Represents a card that has been selected by a [CardInteraction]
pub struct SelectedCard<'r, E: Debug, COMM: Interface<Error = E>> {
    reader: &'r mut Mfrc522<COMM, Initialized>,
    uid: Uid,
}

impl<'r, E: Debug, COMM: Interface<Error = E>> SelectedCard<'r, E, COMM> {
    pub fn auth_sector(&'r mut self, sector: u8, key: &MifareKey) -> Result<AuthenticatedSector<'r, E, COMM>, Error<E>> {
        let block = sector * 4;
        self.reader.mf_authenticate(&self.uid, block, key).map_err(|err| Error::ReaderError(err))?;

        Ok(AuthenticatedSector { reader: self.reader, _uid: &self.uid, sector })
    }

    pub fn uid(&self) -> &Uid {
        &self.uid
    }
}

pub struct AuthenticatedSector<'r, E: Debug, COMM: Interface<Error = E>> {
    reader: &'r mut Mfrc522<COMM, Initialized>,
    _uid: &'r Uid,
    sector: u8,
}

impl<'r, E: Debug, COMM: Interface<Error = E>> AuthenticatedSector<'r, E, COMM> {
    /// Read one of the blocks in the currently authenticated card sector
    pub fn read_block(&mut self, block: u8) -> Result<[u8; BLOCK_SIZE as usize], Error<E>> {
        if block >= 4 {Err(Error::OutOfBounds)?}
        let block = self.sector * 4 + block;

        let read = self.reader.mf_read(block).map_err(|err| Error::ReaderError(err))?;

        Ok(read)
    }

    /// Read all blocks in the currently authenticated card sector
    pub fn read_sector(&mut self) -> Result<[u8; BLOCK_SIZE as usize * SECTOR_SIZE as usize], Error<E>> {
        let mut sector = [0u8; BLOCK_SIZE as usize * SECTOR_SIZE as usize];

        for i in 0..SECTOR_USIZE {
            let block = &mut sector[i*BLOCK_USIZE..(i+1)*BLOCK_USIZE];

            block.copy_from_slice(&self.read_block(i as u8)?);
        }

        Ok(sector)
    }

    /// Write to one of the blocks in the currently authenticated sector. Does not allow a write to the last block in the sector
    pub fn write_block(&mut self, block: u8, data: [u8; BLOCK_USIZE]) -> Result<(), Error<E>> {
        if block >= SECTOR_SIZE {Err(Error::OutOfBounds)?};
        if block == SECTOR_SIZE-1 {Err(Error::SectorTrailerLock)?};

        let block = self.sector * SECTOR_SIZE + block;

        self.reader.mf_write(block, data).map_err(|err| Error::ReaderError(err))?;

        Ok(())
    }

    /// Write to all blocks except the last one in the currently authenticated card sector
    pub fn write_sector(&mut self, data: [u8; BLOCK_USIZE * (SECTOR_USIZE-1)]) -> Result<(), Error<E>> {
        let (chunks, remainder) = data.as_chunks();

        debug_assert!(remainder.is_empty());

        for i in 0..SECTOR_USIZE {
            self.write_block(i as u8, chunks[i])?;
        }

        Ok(())
    }
}

impl<'r, E: Debug, COMM: Interface<Error = E>> Drop for AuthenticatedSector<'r, E, COMM> {
    fn drop(&mut self) {
        self.reader.stop_crypto1();
    }
}

//type Block = [u8; BLOCK_SIZE as usize];

pub const BLOCK_SIZE: u8 = 16;
pub const SECTOR_SIZE: u8 = 4;
pub const CARD_SIZE: u8 = 16;

pub const BLOCK_USIZE: usize = BLOCK_SIZE as usize;
pub const SECTOR_USIZE: usize = SECTOR_SIZE as usize;
pub const CARD_USIZE: usize = CARD_SIZE as usize;

type CardKeys = [MifareKey; CARD_USIZE];