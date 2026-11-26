// The `else` runs where some condition doesn't hold, so it knows none of
// them: `i < 4` isn't known there.
fn at(a: [u8; 4], o: Option<u8>, i: usize) -> u8 with panic {
    if let Some(_) = o && i < 4 { a[i] } else { a[i] }
}

fn main() {
    let _ = at([1, 2, 3, 4], None, 9);
}
// error: can't prove `i < self.len()`
