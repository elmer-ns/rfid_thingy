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
use rfid_thingy::{self as lib, CardData, ReaderInteraction, State, rfid::SECTOR_SIZE};
use esp_hal::time::Rate;
use esp_hal::rmt::Rmt;
use esp_hal_smartled::smart_led_buffer;
use smart_leds_trait::SmartLedsWrite;

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

    let mut led_buffer = smart_led_buffer!(1);

    let mut onboard_led = {
        let frequency = Rate::from_mhz(80);
        let rmt = Rmt::new(peripherals.RMT, frequency).expect("Failed to initialize RMT0");
        esp_hal_smartled::SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO38, &mut led_buffer)
    };

    onboard_led.write(brightness([RED].into_iter(), 10)).unwrap();

    loop {
        let state = &rfid_thingy::STATE;

        let active = state.lock(|state| {
            state.reader_active
        });

        if !active {
            continue;
        }

        log::info!("Waiting for card...");

        let mut card = match reader.wait_for_card().await {
            Ok(card) => card,
            Err(err) => {
                log::error!("Wait error: {:?}", err);
                continue;
            },
        };

        let mut selected = match card.select() {
            Ok(selected) => selected,
            Err(err) => {
                log::error!("Select error: {:?}", err);
                continue;
            },
        };

        let dyn_uid: lib::Uid = selected.uid().into();

        log::info!("Found and selected card");

        let op = state.lock(|state| {
            state.reader_operation.clone()
        });

        let interaction = match op {
            rfid_thingy::ReaderOperation::None => {
                ReaderInteraction::Found { uid: dyn_uid}
            },
            rfid_thingy::ReaderOperation::Read { block, read_sector, key } => {
                let mut auth = match selected.auth_sector(block / SECTOR_SIZE, &key) {
                    Ok(auth) => auth,
                    Err(err) => {
                        log::error!("Auth error: {:?}", err);
                        continue;
                    },
                };
                
                let data = if read_sector {
                    auth.read_sector().map(|sector| CardData::Sector(sector))
                } else {
                    let local_block = block % 4;
                    auth.read_block(local_block).map(|block| CardData::Block(block))
                };

                let data = match data {
                    Ok(data) => data,
                    Err(err) => {
                        log::error!("Read error: {:?}", err);
                        continue;
                    },
                };

                ReaderInteraction::Read { uid: dyn_uid, block, data }
            },
            rfid_thingy::ReaderOperation::Write { block, data, key } => {
                todo!()
            },
        };

        Timer::after(Duration::from_secs(1)).await;
    }
}

/// Initialize the RFID reader. Simple helper function that takes required perhiperals and pins and produces a [Mfrc522] wrapped in a [Reader].
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
