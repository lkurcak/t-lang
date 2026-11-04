// A handle can be joined by another thread than the one that started it.
use std::thread::JoinHandle;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let inner: JoinHandle<'_, u32> = os.threads.spawn(|| 20u32);
    let outer = os.threads.spawn(move || inner.join() + 1);
    let mut out = os.stdout;
    out.line(&outer.join())?; // expect: 21
    Result::Ok(())
}
