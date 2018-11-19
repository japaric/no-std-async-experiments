#![deny(unsafe_code)]
#![feature(generators)]
#![feature(never_type)]
#![no_main]
#![no_std]

// panicking behavior
extern crate panic_semihosting;

use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::hprintln;
use executor::Executor;
use signal::Signal;

#[entry]
fn main() -> ! {
    let p = cortex_m::Peripherals::take().unwrap();

    // configures the system timer to trigger a SysTick exception every second
    let mut syst = p.SYST;
    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(12_000_000); // period = 1s
    syst.enable_counter();
    syst.enable_interrupt();

    let mut t1 = || {
        loop {
            hprintln!("T1").unwrap();

            // Sleep this task until signal A is received
            yield A::id();
        }
    };

    let mut t2 = || {
        loop {
            hprintln!("T2").unwrap();

            // Sleep this task until signal B is received
            yield B::id();
        }
    };

    let mut executor = Executor::new();
    executor.spawn(&mut t1).ok();
    executor.spawn(&mut t2).ok();

    executor.run()
}

#[exception]
fn SysTick() {
    static mut COUNT: u8 = 0;

    *COUNT += 1;

    // Send signal A every second
    A::set();
    if *COUNT % 2 == 0 {
        // Send signal B every 2 seconds
        B::set();
    }
}

#[derive(Signal)]
struct A;

#[derive(Signal)]
struct B;
