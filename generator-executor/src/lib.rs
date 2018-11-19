#![feature(generator_trait)]
#![feature(never_type)]
#![feature(pin)]
#![no_std]

use core::{
    hint,
    ops::{Generator, GeneratorState},
};

use cortex_m::asm;
use heapless::{consts, spsc::Queue, FnvIndexMap, Vec};
use signal::{Id, Signals};

// For simplicity we hard-code the capacity of the Executor
type SIZE = consts::U8;

pub struct Executor<'a> {
    // FIXME(rust-lang/rust#55704) these should be pinned generators
    generators: Vec<&'a mut Generator<Yield = Id, Return = !>, SIZE>,
    /// Routes signals to tasks
    // XXX this could be replaced with `[Option<u8>; 32]` to get faster execution time (no hashing)
    // at the expense of using more stack memory
    router: FnvIndexMap<Id, u8, SIZE>,
    /// Ready tasks
    ready: Queue<u8, SIZE>,
}

impl<'a> Executor<'a> {
    pub fn new() -> Self {
        Executor {
            generators: Vec::new(),
            router: FnvIndexMap::new(),
            ready: Queue::new(),
        }
    }

    pub fn spawn(&mut self, gen: &'a mut Generator<Yield = Id, Return = !>) -> Result<(), ()> {
        let id = self.generators.len();
        self.generators.push(gen).map_err(drop)?;
        unsafe {
            // NOTE(enqueue_unchecked) we can never exceed the capacity of `ready`
            self.ready
                .enqueue_unchecked(id as u8)
        }
        Ok(())
    }

    pub fn run(&mut self) -> ! {
        loop {
            // dispatch ready tasks
            while let Some(i) = self.ready.dequeue() {
                unsafe {
                    if let GeneratorState::Yielded(id) =
                        self.generators.get_unchecked_mut(usize::from(i)).resume()
                    {
                        // NOTE(unchecked_unreachable) we can never exceed the capacity of `router`
                        self.router
                            .insert(id, i as u8)
                            .unwrap_or_else(|_| hint::unreachable_unchecked());
                    }
                }
            }

            let mut signals;
            loop {
                signals = Signals::read();

                if !signals.is_empty() {
                    break;
                }

                asm::wfe();
            }

            for id in signals {
                if let Some(i) = self.router.remove(&id) {
                    unsafe {
                        // NOTE(enqueue_unchecked) we can never exceed the capacity of `ready`
                        self.ready.enqueue_unchecked(i);
                    }
                }
            }
        }
    }
}
