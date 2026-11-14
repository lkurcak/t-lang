// A handle can be joined by another thread than the one that started it.
// What it gives is what its closure did (docs/74): 20, so `+ 1` can't
// overflow, and nothing here can panic.
use std::thread::JoinHandle;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let inner: JoinHandle<'_, u32> = os.threads.spawn(|| 20u32);
    let outer = os.threads.spawn(move || inner.join() + 1);
    let mut out = os.stdout;
    out.line(&outer.join())?; // expect: 21
    Ok(())
}
