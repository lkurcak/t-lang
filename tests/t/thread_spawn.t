// A thread is started with `spawn`, which needs the program's `Threads`,
// and gives a handle that must be joined: `join` waits for the thread and
// gives back what its closure did. The closure may borrow what outlives
// the handle, as the thread is over before the handle is.
use core::num::wrapping_add;

fn sum(xs: &[i64]) -> i64 {
    let mut s = 0i64;
    for x in xs.iter() {
        s = wrapping_add(s, *x);
    }
    s
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let data = [1i64, 2, 3, 4];
    let h = os.threads.spawn(|| sum(data.as_slice()));
    let mine = sum(data.as_slice());
    let theirs = h.join();
    let mut out = os.stdout;
    out.line(&mine)?;    // expect: 10
    out.line(&theirs)?;  // expect: 10
    out.line(&data[0])?; // expect: 1
    Ok(())
}
