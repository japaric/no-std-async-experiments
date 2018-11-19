# `no-std-async-experiments`

> Experiments in `no_std` cooperative multitasking

This repository contains two executors: `futures-executor` and
`generator-executor`. The former uses the `core::{future,task}` machinery; the
latter builds on top of the `Generator` trait.

Both executors support running up to N -- for simplicity, we currently hardcode
N = 8 -- concurrent "infinite" tasks. Both executors will sleep (see WFI and WFE
instructions) the microcontroller when no task can make progress. Interrupts,
and other tasks, can wake up other tasks using a lightweight signaling mechanism
(see the `cortex-m-signal` crate).

Each executor has a simple example (`futures-app` and `generator-app`) which you
can run on your laptop / PC using QEMU. You can run the examples using these
commands:

``` console
$ # install `qemu-system-arm`
$ sudo apt-get install qemu-system-arm

$ cd futures-app
$ cargo run --release
T2
T1
T1
T2
T1
T1
T2
T1
T1
```

## Observations

The `generator-executor` has less levels of indirection and produces smaller
binaries. I haven't measure execution time on a device but I expect it'll run
faster than `futures-executor`.

I expect that async / await functionality will be possible to implement on top
of `generator-executor` without any sort of TLS (Thread Local Storage) or
`static` variable. (`async fn` uses `future::from_generator` which internally
uses a thread-local variable for the waker).

## TODO list

(but don't hold your breath)

- Prototype out of tree async / await functionality.

- Figure out how to make an interrupt wake up a task *without* context switching
  to the interrupt handler. (Can we use interrupt masking and NVIC.ISPR,
  somehow?)

- Support for finite tasks, but these need `Box`-ing.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
