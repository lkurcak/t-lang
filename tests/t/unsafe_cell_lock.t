// A spin lock written in the program over `Unsafe` and an atomic, with its
// own `unsafe impl Sync`, shared by two threads.
use core::cell::Unsafe;
use core::num::wrapping_add;
use core::sync::{Atomic, Send, Sync};
use std::thread::{JoinHandle, Threads};

#[derive(Discard where T: Discard, Send where T: Send)]
struct Lock<T> { held: Atomic<bool>, value: Unsafe<T> }

unsafe impl<T: Send> Sync for Lock<T> {}

impl<T> Lock<T> {
    fn new(value: T) -> Lock<T> { Lock { held: Atomic::new(false), value: Unsafe::new(value) } }

    fn lock<R, F: FnOnce(&mut T) -> R>(&self, f: F) -> R {
        while self.held.swap(true) {}
        // SAFETY: `held` is `true` only while one thread is here.
        let r = f(unsafe { self.value.as_mut() });
        self.held.store(false);
        r
    }

    fn into_inner(self) -> T { self.value.into_inner() }
}

fn add<'a>(threads: &Threads, l: &'a Lock<i64>, by: i64) -> JoinHandle<'a, ()> with alloc {
    threads.spawn(move || {
        let mut i = 0u32;
        while i < 40 {
            l.lock(|n: &mut i64| *n = wrapping_add(*n, by));
            i = i + 1;
        }
    })
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let l = Lock::new(0i64);
    let a = add(&os.threads, &l, 1);
    let b = add(&os.threads, &l, 100);
    a.join();
    b.join();
    let mut out = os.stdout;
    out.line(&l.into_inner())?; // expect: 4040
    Ok(())
}
