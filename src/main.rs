#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(asm)]
#![feature(naked_functions)]
// pick a panicking behavior
//extern crate panic_halt;
// you can put a breakpoint on `rust_begin_unwind` to catch panics
// extern crate panic_abort; // requires nightly
// extern crate panic_itm; // logs messages over ITM; requires ITM support
extern crate panic_semihosting;
// logs messages to the host stderr; requires a debugger
#[macro_use]
extern crate lazy_static;
extern crate alloc;

use cortex_m::peripheral::syst;
use cortex_m::register::control::Spsel;
use cortex_m::{asm, interrupt, register, Peripherals};
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::{debug, hprintln};

mod config;
mod memory;
mod thread;

#[entry]
fn main() -> ! {
    memory::init();
    thread::init();
    loop {
        asm::nop();
    }
}

#[exception]
unsafe fn SysTick() {}
