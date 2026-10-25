// After a branch that may have changed `s`, it's whichever slice the branch
// taken gave it, even if the first branch left it as it was.
fn f(a: &[i64], b: &[i64], c: bool) -> i64 {
    let mut s = a;
    if a.len() > 1 {
        if c {
        } else {
            s = b;
        }
        return s[1];
    }
    0
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: alloc::list::List<i64> = alloc::list::List::new();
    l.push(1);
    l.push(2);
    out.line(&f(l.as_slice(), core::slice::empty(), false))?;
    Result::Ok(())
}
// error: can't prove `i < self.len()` for argument `i` of `index`
