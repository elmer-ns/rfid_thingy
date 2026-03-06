#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::{clock::CpuClock, i2c::master::{Config, I2c}, main};
use log::{error, info};

esp_bootloader_esp_idf::esp_app_desc!();

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);

    loop {}
}

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let mut i2c = I2c::new(peripherals.I2C0, Config::default()).unwrap().with_sda(peripherals.GPIO6).with_scl(peripherals.GPIO7);

    info!("Begin scan");

    let mut c = 0;

    for addr in 0..=127 {
        let mut buffer = [0; 8];
        let read = i2c.read(addr, &mut buffer);

        if read.is_ok() {
            info!("{}: Found I2C Device", addr as u8);
            c += 1;
        } else {
            let err = read.err().unwrap();
            error!("{}: {}", addr as u8, err);
        }
    }

    info!("Scan done, found {} devices", c);

    loop {}
}