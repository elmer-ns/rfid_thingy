use core::{convert::Infallible, fmt::Debug};

use bytemuck::{Pod, Zeroable};
use embedded_hal_bus::spi::{DeviceError, ExclusiveDevice};
use esp_hal::{Blocking, delay::Delay, gpio::{Level, Output, OutputConfig, OutputPin, interconnect::{PeripheralInput, PeripheralOutput}}, spi::{self, master::{Config, Spi}}};
use mfrc522::{MifareKey, comm::{Interface, blocking::spi::SpiInterface}};

use crate::rfid::{BLOCK_USIZE, CARD_USIZE, Error, Reader, SAFE_SECTOR_USIZE, SECTOR_SIZE, SECTOR_USIZE, SelectedCard};

/// Initialize the RFID reader. Simple helper function that takes required perhiperals and pins and produces a [Mfrc522] wrapped in a [Reader].
pub fn init_reader<'d>(
    spi: impl spi::master::Instance + 'd,
    sck: impl PeripheralOutput<'d>,
    mosi: impl PeripheralOutput<'d>,
    miso: impl PeripheralInput<'d>,
    cs: impl OutputPin + 'd,
) -> Option<
    Reader<
        DeviceError<esp_hal::spi::Error, Infallible>,
        SpiInterface<
            ExclusiveDevice<Spi<'d, Blocking>, Output<'d>, Delay>,
            mfrc522::comm::blocking::spi::DummyDelay,
        >,
    >,
> {
    let spi: Spi<'_, esp_hal::Blocking> = Spi::new(spi, Config::default())
        .unwrap()
        .with_sck(sck)
        .with_mosi(mosi)
        .with_miso(miso);

    let cs_pin = Output::new(cs, Level::Low, OutputConfig::default());

    let device = ExclusiveDevice::new(spi, cs_pin, Delay::new()).unwrap();
    let itf = SpiInterface::new(device);

    Reader::new(itf)
}

pub fn read_block<'s, 'r, E: Debug, COMM: Interface<Error = E>>(selected: &'s mut SelectedCard<'r, E, COMM>, block: u8, key: &MifareKey) -> Result<[u8; BLOCK_USIZE], Error<E>> {
    let mut auth =  selected.auth_sector(block / SECTOR_SIZE, key)?;

    let data = auth.read_block(block % 4)?;

    Ok(data)
}

pub fn read_sector<'s, 'r, E: Debug, COMM: Interface<Error = E>>(selected: &'s mut SelectedCard<'r, E, COMM>, sector: u8, key: &MifareKey) -> Result<[u8; BLOCK_USIZE*SECTOR_USIZE], Error<E>> {
    let mut auth =  selected.auth_sector(sector, key)?;

    let data = auth.read_sector()?;

    Ok(data)
}

pub fn read_card<'s, 'r, E: Debug, COMM: Interface<Error = E>>(selected: &'s mut SelectedCard<'r, E, COMM>, keys: &[MifareKey; CARD_USIZE]) -> Result<[u8; BLOCK_USIZE*SECTOR_USIZE*CARD_USIZE], Error<E>> {
    let mut data = [0u8; BLOCK_USIZE*SECTOR_USIZE*CARD_USIZE];

    const STRIDE: usize = BLOCK_USIZE*SECTOR_USIZE; 
    for (i, key) in keys.iter().enumerate() {
        let sector_data = read_sector(selected, i as u8, key)?;

        let src = &sector_data;
        let dst = &mut data[i*STRIDE..(i+1)*STRIDE];

        dst.copy_from_slice(src);
    }

    Ok(data)
}

pub fn write_block<'s, 'r, E: Debug, COMM: Interface<Error = E>>(selected: &'s mut SelectedCard<'r, E, COMM>, block: u8, key: &MifareKey, data: [u8; BLOCK_USIZE]) -> Result<(), Error<E>> {
    let mut auth = selected.auth_sector(block / SECTOR_SIZE, key)?;

    auth.write_block(block%4, data)?;

    Ok(())
}

pub fn write_sector<'s, 'r, E: Debug, COMM: Interface<Error = E>>(selected: &'s mut SelectedCard<'r, E, COMM>, block: u8, key: &MifareKey, data: [u8; BLOCK_USIZE*SAFE_SECTOR_USIZE]) -> Result<(), Error<E>> {
    let mut auth = selected.auth_sector(block / SECTOR_SIZE, key)?;

    auth.write_sector(data)?;

    Ok(())
}

#[repr(transparent)]
#[derive(Clone, Copy)]
struct CardDataSingle([u8; BLOCK_USIZE*SAFE_SECTOR_USIZE*CARD_USIZE]);

unsafe impl Zeroable for CardDataSingle {}
unsafe impl Pod for CardDataSingle {}

pub fn write_card<'s, 'r, E: Debug, COMM: Interface<Error = E>>(selected: &'s mut SelectedCard<'r, E, COMM>, block: u8, key: &MifareKey, data: [u8; BLOCK_USIZE*SAFE_SECTOR_USIZE*CARD_USIZE]) -> Result<(), Error<E>> {
    let chunks: [[u8; BLOCK_USIZE*SAFE_SECTOR_USIZE]; CARD_USIZE] = bytemuck::cast(CardDataSingle(data));
    
    for (i, chunk) in chunks.into_iter().enumerate() {
        write_sector(selected, block, key, chunk)?;
    }

    Ok(())
}