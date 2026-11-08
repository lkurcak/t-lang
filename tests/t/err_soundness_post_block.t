// A `&mut` argument that isn't a place, `{ &mut l }`, lends what's behind
// it all the same: after the call it's a value nothing's known of, not
// the one before, of which `push` would say `l.len() == l.len() + 1`.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut l: List<u8> = List::new();
    List::push({ &mut l }, 0u8);
    let a: [u8; 1] = [7];
    out.line(&a[5])?;
    Ok(())
}
// error: can't prove `i < self.len()` for argument `i` of `index`
