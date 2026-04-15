#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_hal_bus::spi::{DeviceError, ExclusiveDevice};
use esp_hal::Blocking;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::interconnect::{PeripheralInput, PeripheralOutput};
use esp_hal::gpio::{Level, Output, OutputConfig, OutputPin};
use esp_hal::spi::master::{Config, Instance, Spi};
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use log::info;
use mfrc522::comm::blocking::spi::SpiInterface;
use rfid_thingy::rfid::Reader;

use esp_backtrace as _;

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

    

    
    let mut reader = init_reader(peripherals.SPI2, peripherals.GPIO9, peripherals.GPIO11, peripherals.GPIO12, peripherals.GPIO10).unwrap();

    loop {
        info!("waiting...");

        let Ok(mut card) = reader.wait_for_card().await else {continue;};
        let Ok(mut select) = card.select() else {continue;};
        info!("found card!");

        let Ok(mut auth_sector) = select.auth_sector(0, &[0xFF; 6]) else {continue;};
        info!("authenticated card");

        let Ok(sector) = auth_sector.read_sector() else{ continue;};

        println!("read: {:?}", sector);

        let Ok(_) = auth_sector.write_block(1, [6,7,6,5,6,7,6,7,6,7,6,7,6,7,6,7]) else {continue;};

        let Ok(sector) = auth_sector.read_sector() else{ continue;};

        println!("read: {:?}", sector);

        Timer::after(Duration::from_secs(1)).await;
    }
}

fn init_reader<'d>(spi: impl Instance + 'd, sck: impl PeripheralOutput<'d>, mosi: impl PeripheralOutput<'d>, miso: impl PeripheralInput<'d>, cs: impl OutputPin + 'd) -> Option<Reader<DeviceError<esp_hal::spi::Error, Infallible>, SpiInterface<ExclusiveDevice<Spi<'d, Blocking>, Output<'d>, Delay>, mfrc522::comm::blocking::spi::DummyDelay>>> {
    let spi: Spi<'_, esp_hal::Blocking> = Spi::new(spi, Config::default()).unwrap().with_sck(sck).with_mosi(mosi).with_miso(miso);
    
    let cs_pin = Output::new(cs, Level::Low, OutputConfig::default());

    let device = ExclusiveDevice::new(spi, cs_pin, Delay::new()).unwrap();
    let itf = SpiInterface::new(device);

    Reader::new(itf)
}