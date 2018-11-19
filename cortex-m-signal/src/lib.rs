//! Interrupt-safe, thread-safe signaling mechanism
//!
//! **IMPORTANT**: This crate is only sound if the target is an ARM Cortex-M device that supports
//! bit banding.
//!
//! The main use case for this crate is efficiently waking up the main "thread" from an interrupt
//! handler
//!
//! ``` ignore
//! // Use the `derive(Signal)` attribute to create a new signal
//! #[derive(Signal)]
//! struct A;
//!
//! #[derive(Signal)]
//! struct B;
//!
//! #[entry]
//! fn main() -> ! {
//!     loop {
//!         // Iterate over the signals that were set
//!         for id in Signals::read() {
//!              if id == A::id() {
//!                  // woken up by SysTick
//!                  // ..
//!              } else if id == B::id() {
//!                  // woken up by TIM6
//!                  // ..
//!              }
//!         }
//!
//!         // sleep until the next signal is received
//!         asm::wfi();
//!     }
//! }
//!
//! #[exception]
//! fn SysTick() {
//!     // ..
//!     // Wake up `main`
//!     A::set();
//!     // ..
//! }
//!
//! #[interrupt]
//! fn TIM6() {
//!     // ..
//!     // Wake up `main`
//!     B::set();
//!     // ..
//! }
//! ```
//!
//! This is much cheaper than setting one SPSC queue between each interrupt handler and `main`. This
//! is also more efficient than setting up one `AtomicBool` per interrupt. See below:
//!
//! ``` ignore
//! static A: AtomicBool = AtomicBool::new(false);
//! static B: AtomicBool = AtomicBool::new(false);
//!
//! #[entry]
//! fn main() -> ! {
//!     loop {
//!         if A.swap(false, Ordering::Relaxed) {
//!             // woken up by SysTick
//!             // ..
//!
//!         }
//!
//!         if B.swap(false, Ordering::Relaxed) {
//!             // woken up by TIM6
//!             // ..
//!         }
//!
//!         // sleep until the next signal is received
//!         asm::wfi();
//!     }
//! }
//!
//! #[exception]
//! fn SysTick() {
//!     // ..
//!     // Wake up `main`
//!     A.store(true, Ordering::Relaxed);
//!     // ..
//! }
//!
//! #[interrupt]
//! fn TIM6() {
//!     // ..
//!     // Wake up `main`
//!     B.store(true, Ordering::Relaxed);
//!     // ..
//! }
//!
//! ```

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]

use core::sync::atomic::{AtomicUsize, Ordering};

use hash32_derive::Hash32;
pub use signal_macros::Signal;

/// A signal identifier
#[derive(Clone, Copy, Debug, Eq, Hash32, PartialEq)]
pub struct Id(u8);

impl From<Id> for u8 {
    fn from(id: Id) -> u8 {
        id.0
    }
}

/// A snapshot of the signals that *were* set
#[derive(Clone, Copy, Debug)]
pub struct Signals(usize);

impl Signals {
    /// Returns a snapshot of the signals currently set
    ///
    /// **NOTE**: this will clear all the currently set signals
    pub fn read() -> Self {
        // NOTE(Ordering::Relaxed) we assume a single core target
        Signals(SIGNALS.swap(0, Ordering::Relaxed))
    }

    /// Returns `true` if no signal is set in this snapshot
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

impl Iterator for Signals {
    type Item = Id;

    fn next(&mut self) -> Option<Id> {
        if self.0 == 0 {
            None
        } else {
            let pos = 31 - self.0.leading_zeros() as u8;
            self.0 &= !(1 << pos);
            Some(Id(pos))
        }
    }
}

static SIGNALS: AtomicUsize = AtomicUsize::new(0);

// Bit banding
const RAM_START: usize = 0x2000_0000;
const ALIAS_START: usize = 0x2200_0000;

/// A unique signal type
pub unsafe trait Signal {
    /// The identifier for this signal
    fn id() -> Id {
        Id(Self::usize() as u8)
    }

    /// IMPLEMENTATION DETAIL. DO NOT USE
    #[doc(hidden)]
    fn ptr() -> *const AtomicUsize {
        let id = Self::usize();
        let p = &SIGNALS as *const AtomicUsize as usize;
        ((32 * p.wrapping_sub(RAM_START)).wrapping_add(ALIAS_START) + 4 * id) as *const AtomicUsize
    }

    /// Sets this signal
    fn set() {
        unsafe {
            // NOTE(Ordering::Relaxed) we assume a single core target
            AtomicUsize::store(&*Self::ptr(), 1, Ordering::Relaxed);
        }
    }

    /// IMPLEMENTATION DETAIL. DO NOT USE
    #[doc(hidden)]
    fn usize() -> usize;
}
