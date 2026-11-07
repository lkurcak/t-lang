// A write through a `&mut` to the payload could break the refinement.
fn main() {
    let mut o: Option<{ v: u8 | v < 3 }> = Some(1);
    if let Some(x) = &mut o {
        *x = 7;
    }
}
// error: can't borrow `o` as `&mut`
