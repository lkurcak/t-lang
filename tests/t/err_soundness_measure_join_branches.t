// After an `if`, a place each branch left a different object at is a new
// one, whose measures are, on each path, that path's: here `l.len()` is 1
// or 0, so `l[0]` can't be proved.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    let flag = l.is_empty();
    if flag {
        l.push(1);
    } else {
        l.clear();
    }
    out.line(&l[0])?;
    Result::Ok(())
}
// error: can't prove `i < self.len()` for argument `i` of `index`
