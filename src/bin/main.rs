#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]


use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_hal_bus::spi::{self, DeviceError, ExclusiveDevice};
use embedded_io::Read;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig, OutputPin, Pin};
use esp_hal::peripherals::GPIO;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::timer::timg::TimerGroup;
use log::{info, warn};
use mfrc522::Mfrc522;
use mfrc522::comm::blocking::i2c::I2cInterface;
use mfrc522::comm::blocking::spi::SpiInterface;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
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

    let mut mfrc522: Mfrc522<SpiInterface<ExclusiveDevice<Spi<'_, esp_hal::Blocking>, Output<'_>, Delay>, mfrc522::comm::blocking::spi::DummyDelay>, mfrc522::Initialized> = Mfrc522::new(itf).init().unwrap();

    let mfrc522_version = mfrc522.version().unwrap();

    info!("mfrc522_version={}", mfrc522_version);

    loop {
        if new_card_present(&mut mfrc522) {
            info!("new card!");
        }

        Timer::after(Duration::from_secs(1)).await;
    }
}

type Reader<'a> = Mfrc522<SpiInterface<ExclusiveDevice<Spi<'a, esp_hal::Blocking>, Output<'a>, Delay>, mfrc522::comm::blocking::spi::DummyDelay>, mfrc522::Initialized>;

fn new_card_present(reader: &mut Reader) -> bool {
    match reader.new_card_present() {
        Ok(_) => true,
        Err(_) => false,
    }
}