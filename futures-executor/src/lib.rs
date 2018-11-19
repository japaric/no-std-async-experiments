#![feature(arbitrary_self_types)]
#![feature(futures_api)]
#![feature(never_type)]
#![feature(pin)]
#![no_std]

use core::{
    cell::{RefCell, UnsafeCell},
    future::Future,
    hint,
    pin::Pin,
    ptr::NonNull,
    task::{LocalWaker, UnsafeWake, Waker},
};

use cortex_m::asm;
use heapless::{consts, spsc::Queue, FnvIndexMap, Vec};
use signal::{Id, Signals};

// For simplicity we hardcode the capacity of the executor
//
// The capacity could be generic but the trait bounds are annoying to write
type SIZE = consts::U8;

/// Routes signals to tasks
pub struct Router {
    wakers: RefCell<FnvIndexMap<Id, LocalWaker, SIZE>>,
}

impl Router {
    #[inline]
    pub fn new() -> Self {
        Router {
            wakers: RefCell::new(FnvIndexMap::new()),
        }
    }

    /// Routes the given `signal` to the given `waker`
    ///
    /// When the `signal` is set the `waker` will be used to wake up a task
    #[inline]
    pub fn route(&self, signal: Id, waker: LocalWaker) {
        self.wakers.borrow_mut().insert(signal, waker).ok();
    }

    #[inline]
    fn wake(&self, signal: Id) {
        if let Some(waker) = self.wakers.borrow_mut().remove(&signal) {
            waker.wake();
        }
    }
}

struct Task {
    id: u8,
    // XXX kind of wasteful because all `Task`s will have the same pointer
    ready_queue: NonNull<UnsafeCell<Queue<u8, SIZE>>>,
}

// HACK Task is NOT Send or Sync but the UnsafeWake trait requires these so ...
unsafe impl Send for Task {}
unsafe impl Sync for Task {}

unsafe impl UnsafeWake for Task {
    #[inline]
    unsafe fn clone_raw(&self) -> Waker {
        Waker::new(NonNull::from(self as &UnsafeWake))
    }

    #[inline]
    unsafe fn drop_raw(&self) {}

    #[inline]
    unsafe fn wake(&self) {
        unreachable!()
    }

    #[inline]
    unsafe fn wake_local(&self) {
        (*self.ready_queue.as_ref().get()).enqueue_unchecked(self.id)
    }
}

pub struct Executor<'a> {
    // all futures
    futures: Vec<Pin<&'a mut Future<Output = !>>, SIZE>,
    // all "tasks"
    // XXX we only need this because LocalWakers need to point to something ...
    tasks: Vec<Task, SIZE>,
    // queue of ready tasks (IDs only)
    ready_queue: UnsafeCell<Queue<u8, SIZE>>,
    router: &'a Router,
}

impl<'a> Executor<'a> {
    #[inline]
    pub fn new(router: &'a Router) -> Self {
        Executor {
            futures: Vec::new(),
            tasks: Vec::new(),
            ready_queue: UnsafeCell::new(Queue::new()),
            router,
        }
    }

    /// Spawns the given `fut`-ure as a task
    ///
    /// Note that the task won't start or make progress until `run` is called
    #[inline]
    pub fn spawn(mut self: Pin<&mut Self>, fut: Pin<&'a mut Future<Output = !>>) -> Result<(), ()> {
        let id = self.tasks.len() as u8;

        self.futures.push(fut).map_err(drop)?;

        // NOTE(NonNull) OK because `self` is pinned thus `ready_queue` is also immovable
        let nn = NonNull::from(&self.ready_queue);
        self.tasks
            .push(Task {
                id,
                ready_queue: nn,
            })
            .unwrap_or_else(|_| unsafe { hint::unreachable_unchecked() });

        unsafe {
            (*self.ready_queue.get()).enqueue_unchecked(id);
        }

        Ok(())
    }

    /// Runs all the spawned tasks
    #[inline]
    pub fn run(mut self: Pin<&mut Self>) -> ! {
        loop {
            unsafe {
                // advance ready tasks
                while let Some(id) = (*self.ready_queue.get()).dequeue() {
                    let task = self.tasks.as_ptr().add(usize::from(id)) as *mut Task;

                    // NOTE(NonNull) OK because `self` is pinned thus `tasks` is also immovable
                    let lw = LocalWaker::new(NonNull::new_unchecked(task));

                    self.futures
                        .get_unchecked_mut(usize::from(id))
                        .as_mut()
                        .poll(&lw);
                }

                // wait for a signal
                let mut signals;
                loop {
                    signals = Signals::read();

                    if !signals.is_empty() {
                        break;
                    }

                    asm::wfe();
                }

                for id in signals {
                    self.router.wake(id);
                }
            }
        }
    }
}
