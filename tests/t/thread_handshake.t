// Two threads that wait for each other: each spins until the other has
// said its part. However the threads are run, each waits only for what
// the other does next.
use core::sync::Atomic;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let turn = Atomic::new(0u32);
    let ping = os.threads.spawn(|| {
        let mut said = 0u32;
        while said < 3 {
            if turn.load() == said * 2 {
                turn.store(said * 2 + 1);
                said = said + 1;
            }
        }
        said
    });
    let pong = os.threads.spawn(|| {
        let mut said = 0u32;
        while said < 3 {
            if turn.load() == said * 2 + 1 {
                turn.store(said * 2 + 2);
                said = said + 1;
            }
        }
        said
    });
    let total = ping.join() + pong.join();
    let mut out = os.stdout;
    out.line(&total)?;       // expect: 6
    out.line(&turn.load())?; // expect: 6
    Result::Ok(())
}
