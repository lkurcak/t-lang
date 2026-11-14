// How many threads the machine can run at once, for this program: at
// least one, which the refinement says, so it can be a barrier's count or
// a divisor without a check.
use alloc::list::List;
use core::sync::Barrier;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let n = os.threads.available();
    let b = Barrier::new(n);
    let per = 1000 / n;
    let mut l: List<usize> = List::with_capacity(n);
    l.push(per);
    let _ = b;
    let mut out = os.stdout;
    out.line(&(n >= 1))?; // expect: true
    out.line(&(per <= 1000))?; // expect: true
    Ok(())
}
