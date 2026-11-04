// A mutex in a static: shared by every function, and so by every thread.
// It's `Sync` as long as what's in it is `Send`.
use core::sync::Mutex;

static LOG: Mutex<[u32; 4]> = Mutex::new([0, 0, 0, 0]);

fn note(slot: usize, v: u32) with global {
    LOG.lock(|log: &mut [u32; 4]| {
        if slot < 4 {
            log[slot] = v;
        }
    });
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, global, panic} {
    let a = os.threads.spawn(|| note(0, 7));
    let b = os.threads.spawn(|| note(3, 9));
    a.join();
    b.join();
    let sum = LOG.lock(|log: &mut [u32; 4]| log[0] + log[1] + log[2] + log[3]);
    let mut out = os.stdout;
    out.line(&sum)?; // expect: 16
    Result::Ok(())
}
