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
use embassy_sync::blocking_mutex::Mutex;
use embassy_time::{Duration, Timer};
use embedded_hal_bus::spi::{DeviceError, ExclusiveDevice};
use esp_hal::{
    Blocking,
    clock::CpuClock,
    delay::Delay,
    gpio::{
        Level, Output, OutputConfig, OutputPin,
        interconnect::{PeripheralInput, PeripheralOutput},
    },
    spi::master::{Config, Instance, Spi},
    timer::timg::TimerGroup,
};
use esp_println::println;
use log::info;
use mfrc522::comm::blocking::spi::SpiInterface;

use esp_backtrace as _;

use lib::rfid::Reader;
use rfid_thingy::{self as lib, State};

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

    let client_config = esp_radio::wifi::ClientConfig::default()
        .with_ssid(SSID.into())
        .with_password(PASSWORD.into());

    let net_stack =
        lib::wifi::start_wifi(&radio_init, peripherals.WIFI, rng, &spawner, client_config).await;

    let web_app = lib::web::WebApp::default();
    for id in 0..lib::web::WEB_TASK_POOL_SIZE {
        spawner.must_spawn(lib::web::web_task(
            id,
            net_stack,
            web_app.router,
            web_app.config,
        ));
    }

    // GPIO 09 - SCK
    // GPIO 11 - MOSI
    // GPIO 12 - MISO
    // GPIO 10 - CS
    let mut reader = init_reader(
        peripherals.SPI2,
        peripherals.GPIO9,
        peripherals.GPIO11,
        peripherals.GPIO12,
        peripherals.GPIO10,
    )
    .unwrap();

    loop {
        info!("waiting...");

        let Ok(mut card) = reader.wait_for_card().await else {
            continue;
        };
        let Ok(mut select) = card.select() else {
            continue;
        };
        info!("found card!");

        let Ok(mut auth_sector) = select.auth_sector(0, &[0xFF; 6]) else {
            continue;
        };
        info!("authenticated card");

        let Ok(sector) = auth_sector.read_sector() else {
            continue;
        };

        println!("read: {:?}", sector);

        //let Ok(_) = auth_sector.write_block(1, [6, 7, 6, 5, 6, 7, 6, 7, 6, 7, 6, 7, 6, 7, 6, 7])
        //else {
        //   continue;
        //};

        let Ok(sector) = auth_sector.read_sector() else {
            continue;
        };

        println!("read: {:?}", sector);

        Timer::after(Duration::from_secs(1)).await;
    }
}

fn init_reader<'d>(
    spi: impl Instance + 'd,
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
