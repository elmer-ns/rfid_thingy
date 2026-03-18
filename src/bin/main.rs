#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::fmt::Debug;

use embassy_executor::Spawner;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::spi::master::{Config, Spi};
use esp_hal::timer::timg::TimerGroup;
use log::{info, warn};
use mfrc522::comm::Interface;
use mfrc522::{AtqA, Initialized, Mfrc522, Uid};
use mfrc522::comm::blocking::spi::SpiInterface;
use rfid_thingy::Reader;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    info!("Embassy initialized!");

    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    let spi: Spi<'_, esp_hal::Blocking> = Spi::new(peripherals.SPI2, Config::default()).unwrap().with_sck(peripherals.GPIO9).with_mosi(peripherals.GPIO11).with_miso(peripherals.GPIO12);
    
    let cs_pin = Output::new(peripherals.GPIO10, Level::Low, OutputConfig::default());

    let device = ExclusiveDevice::new(spi, cs_pin, Delay::new()).unwrap();
    let itf = SpiInterface::new(device);

    info!("mfrc522_version={}", mfrc522_version);

    let reader = Reader::new(itf);

    loop {
        //Timer::after(Duration::from_secs(1)).await;
    }
}

fn read_ready<E: Debug, COMM: Interface<Error = E>>(reader: &mut Mfrc522<COMM, Initialized>, uid: &Uid, block: u8, key: &[u8; 6]) -> Option<[u8; 16]> {
    let mut r = [0; 16];

    match authenticate(reader, &uid, block, key,|reader| {
        let read = reader.mf_read(block).unwrap();

        r = read;

        Ok(())
    }) {
        Ok(_) => Some(r),
        Err(_) => None,
    }
}

fn read_once<E: Debug, COMM: Interface<Error = E>>(reader: &mut Mfrc522<COMM, Initialized>, atqa: &AtqA, key: &[u8; 6], block: u8) -> Option<[u8; 16]> {
    let Ok(uid) = reader.select(&atqa) else {return None};

    read_ready(reader, &uid, block, key)
}

fn authenticate<E, COMM: Interface<Error = E>, F: FnOnce(&mut Mfrc522<COMM, Initialized>) -> Result<(), mfrc522::Error<E>>> (reader: &mut Mfrc522<COMM, Initialized>, uid: &Uid, block: u8, key: &[u8; 6], action: F) -> Result<(), mfrc522::Error<E>> {
    //let key = [0xFF; 6];
    
    match reader.mf_authenticate(uid, block, key) {
        Ok(_) => {},
        Err(err) => {
            warn!("Could not authenticate");
            return Result::Err(err);
        },
    }

    action(reader)?;

    reader.hlta()?;
    reader.stop_crypto1()?;
    Ok(())
}