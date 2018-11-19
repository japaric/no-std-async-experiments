#![deny(unsafe_code)]
#![feature(arbitrary_self_types)]
#![feature(futures_api)]
#![feature(never_type)]
#![feature(pin)]
#![no_main]
#![no_std]

// panicking behavior
extern crate panic_semihosting;

use core::{
    future::Future,
    pin::Pin,
    task::{LocalWaker, Poll},
};

use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::{entry, exception};
use cortex_m_semihosting::hprintln;
use executor::{Executor, Router};
use pin_utils::pin_mut;
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

    let router = &Router::new();
    let executor = Executor::new(router);
    pin_mut!(executor);

    let t1 = T1::new(router);
    pin_mut!(t1);
    executor.as_mut().spawn(t1).ok();

    let t2 = T2::new(router);
    pin_mut!(t2);
    executor.as_mut().spawn(t2).ok();

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

struct T1<'a> {
    router: &'a Router,
}

impl<'a> T1<'a> {
    fn new(router: &'a Router) -> Self {
        T1 { router }
    }
}

impl<'a> Future for T1<'a> {
    type Output = !;

    fn poll(self: Pin<&mut Self>, lw: &LocalWaker) -> Poll<!> {
        hprintln!("T1").unwrap();

        // Sleep this task until signal A is received
        self.router.route(A::id(), lw.clone());
        Poll::Pending
    }
}

struct T2<'a> {
    router: &'a Router,
}

impl<'a> T2<'a> {
    fn new(router: &'a Router) -> Self {
        T2 { router }
    }
}

impl<'a> Future for T2<'a> {
    type Output = !;

    fn poll(self: Pin<&mut Self>, lw: &LocalWaker) -> Poll<!> {
        hprintln!("T2").unwrap();

        // Sleep this task until signal B is received
        self.router.route(B::id(), lw.clone());
        Poll::Pending
    }
}

#[derive(Signal)]
struct A;

#[derive(Signal)]
struct B;
