#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::fmt::Debug;
use core::panic;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::spi::master::{Config, Spi};
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
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

    //let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    //let (mut _wifi_controller, _interfaces) =
    //    esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
    //        .expect("Failed to initialize Wi-Fi controller");

    let spi: Spi<'_, esp_hal::Blocking> = Spi::new(peripherals.SPI2, Config::default()).unwrap().with_sck(peripherals.GPIO9).with_mosi(peripherals.GPIO11).with_miso(peripherals.GPIO12);
    
    let cs_pin = Output::new(peripherals.GPIO10, Level::Low, OutputConfig::default());

    let device = ExclusiveDevice::new(spi, cs_pin, Delay::new()).unwrap();
    let itf = SpiInterface::new(device);

    let mut reader = Reader::new(itf).unwrap();

    loop {
        info!("ready!");

        let atqa = reader.new_card_present_async().await.unwrap();

        info!("new card!");

        let Ok(uid) = reader.select(&atqa) else {continue;};

        info!("successfull select");

        let test = reader.read_sector(&uid, 0, &[0xFF; 6]).unwrap();
        println!("{:?}", test);

        let test = reader.read_sector(&uid, 1, &[0xFF; 6]).unwrap();
        println!("{:?}", test);

        warn!("done!");

        Timer::after(Duration::from_secs(1)).await;
    }
}