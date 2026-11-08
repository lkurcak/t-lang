// A call by path, `List::push(&mut l, ..)`, lends `l` as a method call
// does: what `push` ensures is of `l` after it, not before, which would
// say `l.len() == l.len() + 1` and so prove anything (found in step 53;
// in every release since 0.43.0).
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<u8> = List::new();
    List::push(&mut l, 0u8);
    let a: [u8; 1] = [7];
    out.line(&a[5])?;
    Ok(())
}
// error: can't prove `i < self.len()` for argument `i` of `index`
