#![no_std]
#![feature(impl_trait_in_assoc_type)]

use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};

pub mod rfid;
pub mod web;
pub mod wifi;

extern crate alloc;

#[macro_export]
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

static STATE: Mutex<CriticalSectionRawMutex, State> = Mutex::new(State {
    reader_active: todo!(),
    reader_operation: todo!(),
});

pub struct State {
    pub reader_active: bool,
    pub reader_operation: ReaderOperation,
}

pub enum ReaderOperation {
    None,
    Read { _never: ! },
    Write { _never: ! },
}
