// What a `_` arm after string patterns matches has no known length.
fn first(s: &str) -> u8 {
    match s {
        "" => 0,
        _ => s.as_bytes()[0],
    }
}

fn main() {
    let _ = first("");
}
// error: can't prove `i < self.len()` for argument `i` of `index`
