// A mutex lets threads change a value they share, one at a time: `lock`
// waits until no other thread has it, and lends what's in it as `&mut`
// to the closure, for as long as the closure runs.
use core::num::wrapping_add;
use core::sync::Mutex;
use std::thread::{JoinHandle, Threads};

#[derive(Send)]
struct Tally { sum: i64, adds: u32 }

fn add<'a>(threads: &Threads, t: &'a Mutex<Tally>, by: i64) -> JoinHandle<'a, ()> with alloc {
    threads.spawn(move || {
        let mut i = 0u32;
        while i < 50 {
            t.lock(|t: &mut Tally| {
                t.sum = wrapping_add(t.sum, by);
                t.adds = wrapping_add(t.adds, 1);
            });
            i = i + 1;
        }
    })
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let t = Mutex::new(Tally { sum: 0, adds: 0 });
    let a = add(&os.threads, &t, 1);
    let b = add(&os.threads, &t, 10);
    let seen = t.lock(|t: &mut Tally| t.adds <= 100);
    a.join();
    b.join();
    let Tally { sum, adds } = t.into_inner();
    let mut out = os.stdout;
    out.line(&seen)?; // expect: true
    out.line(&sum)?;  // expect: 550
    out.line(&adds)?; // expect: 100
    Result::Ok(())
}
