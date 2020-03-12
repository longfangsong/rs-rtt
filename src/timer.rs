use cortex_m::{asm, interrupt, register, Peripherals};

pub fn init() {
    let mut peripherals = Peripherals::take().unwrap();
    let ticks_per_10ms = cortex_m::peripheral::SYST::get_ticks_per_10ms();
    peripherals.SYST.set_reload(ticks_per_10ms);
    peripherals.SYST.enable_interrupt();
}
