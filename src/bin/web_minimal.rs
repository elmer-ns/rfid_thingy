#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_hal_bus::spi::{DeviceError, ExclusiveDevice};
use esp_hal::{Blocking, clock::CpuClock, delay::Delay, gpio::{Level, Output, OutputConfig, OutputPin, interconnect::{PeripheralInput, PeripheralOutput}}, spi::master::{Config, Instance, Spi}, timer::timg::TimerGroup};
use esp_println::println;
use esp_radio::wifi::AuthMethod;
use log::info;
use mfrc522::comm::blocking::spi::SpiInterface;

use esp_backtrace as _;

use rfid_thingy as lib;
use lib::rfid::Reader;

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    println!("{}", SSID);
    println!("{}", PASSWORD);

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    info!("Embassy initialized!");

    let radio_init = &*lib::mk_static!(
        esp_radio::Controller<'static>,
        esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller")
    );
    let rng = esp_hal::rng::Rng::new();

    //let client_config = esp_radio::wifi::ClientConfig::default().with_ssid(SSID.into()).with_password(PASSWORD.into());
    let client_config = esp_radio::wifi::ClientConfig::default().with_ssid(SSID.into()).with_password(PASSWORD.into());

    let net_stack = lib::wifi::start_wifi(&radio_init, peripherals.WIFI, rng, &spawner, client_config).await;

    let web_app = lib::web::WebApp::default();
    for id in 0..lib::web::WEB_TASK_POOL_SIZE {
        spawner.must_spawn(lib::web::web_task(
            id,
            net_stack,
            web_app.router,
            web_app.config,
        ));
    }

    loop {
        Timer::after_secs(1).await
    }
}