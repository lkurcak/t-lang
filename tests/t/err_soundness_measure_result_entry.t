// `old(..)` in a result's refinement is the arguments as they were at the
// call: `r` is how long the list was, not how long it is after `pop`.
use alloc::list::List;

fn shrink(l: &mut List<i64>) -> { r: usize | r == old(l.len()) } {
    let n = l.len();
    l.pop();
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    l.push(1);
    let r = shrink(&mut l);
    if r > 0 {
        out.line(&l[r - 1])?;
    }
    Result::Ok(())
}
// error: can't prove `i < self.len()` for argument `i` of `index`
