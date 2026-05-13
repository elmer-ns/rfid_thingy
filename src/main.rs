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

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{clock::CpuClock, timer::timg::TimerGroup};
use esp_println::println;
use log::info;

use esp_backtrace as _;

use esp_hal::rmt::Rmt;
use esp_hal::time::Rate;

use rfid_thingy as lib;

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
    let mut reader = rfid_thingy::helpers::init_reader(
        peripherals.SPI2,
        peripherals.GPIO9,
        peripherals.GPIO11,
        peripherals.GPIO12,
        peripherals.GPIO10,
    )
    .unwrap();

    let frequency: Rate = Rate::from_mhz(80);
    spawner.must_spawn(lib::light_task(
        Rmt::new(peripherals.RMT, frequency).unwrap(),
        peripherals.GPIO38,
    ));

    let mut last_active = false;

    loop {
        let state = &rfid_thingy::STATE;

        {
            let active = state.lock(|state| state.reader_active);

            if active && !last_active {
                log::info!("Activated");
            }

            if !active && last_active {
                log::info!("Deactivated");
            }

            last_active = active;
        }

        Timer::after(Duration::from_secs(1)).await;
    }
    /*
    loop {
        let state = &rfid_thingy::STATE;

        let active = state.lock(|state| state.reader_active);

        if !active {
            continue;
        }

        log::info!("Waiting for card...");

        let mut card = match reader.wait_for_card().await {
            Ok(card) => card,
            Err(err) => {
                log::error!("Wait error: {:?}", err);
                continue;
            }
        };

        let mut selected = match card.select() {
            Ok(selected) => selected,
            Err(err) => {
                log::error!("Select error: {:?}", err);
                continue;
            }
        };

        let dyn_uid: lib::Uid = selected.uid().into();

        log::info!("Found and selected card");

        let op = state.lock(|state| state.reader_operation.clone());

        let interaction = match op {
            rfid_thingy::ReaderOperation::None => ReaderInteraction::Found { uid: dyn_uid },
            rfid_thingy::ReaderOperation::ReadBlock { block, key } => {
                let data = match rfid_thingy::helpers::read_block(&mut selected, block, &key) {
                    Ok(data) => data,
                    Err(err) => {
                        log::error!("{:?}", err);
                        continue;
                    }
                };

                ReaderInteraction::ReadBlock {
                    uid: dyn_uid,
                    block,
                    data,
                }
            }
            rfid_thingy::ReaderOperation::ReadSector { sector, key } => {
                let data = match rfid_thingy::helpers::read_sector(&mut selected, sector, &key) {
                    Ok(data) => data,
                    Err(err) => {
                        log::error!("{:?}", err);
                        continue;
                    }
                };

                ReaderInteraction::ReadSector {
                    uid: dyn_uid,
                    sector,
                    data,
                }
            }
            rfid_thingy::ReaderOperation::ReadCard { keys } => {
                let data = match rfid_thingy::helpers::read_card(&mut selected, &keys) {
                    Ok(data) => data,
                    Err(err) => {
                        log::error!("{:?}", err);
                        continue;
                    }
                };

                ReaderInteraction::ReadCard { uid: dyn_uid, data }
            }
            rfid_thingy::ReaderOperation::WriteBlock { block, key, data } => a,
        };

        Timer::after(Duration::from_secs(1)).await;
    }
    */
}
