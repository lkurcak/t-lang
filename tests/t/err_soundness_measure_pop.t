// Lending a list `&mut` lets the callee change its length: what was known
// of `l.len()` is about the list it was.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<i64> = List::new();
    l.push(1);
    if l.len() > 0 {
        l.pop();
        out.line(&l[0])?;
    }
    Result::Ok(())
}
// error: can't prove `i < self.len()` for argument `i` of `index`
