// A slice can't outlive the list it's of.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    l.push(1);
    let s = l.as_slice();
    l.discard();
    out.line(&s.len())?;
    Result::Ok(())
}
// error: cannot move out of `l` while `l` is borrowed
