// `spin` says a thread is waiting for another: in a loop that looks again
// and again, it lets the machine run another thread, or rest a moment,
// before the next look. It changes nothing, and can't fail, so a function
// that says it does nothing (`with {}`) can call it.
use core::sync::{Atomic, spin};

fn wait_for(flag: &Atomic<bool>) with {} {
    while !flag.load() {
        spin();
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let flag = Atomic::new(false);
    let value = Atomic::new(0u32);
    let h = os.threads.spawn(|| {
        value.store(42);
        flag.store(true);
    });
    wait_for(&flag);
    let seen = value.load();
    h.join();
    let mut out = os.stdout;
    out.line(&seen)?; // expect: 42
    Ok(())
}
