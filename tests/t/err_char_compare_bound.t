// What a comparison of characters says is only what it says: `c <= 'z'`
// is `c as u32 <= 122`, so `c as u32 - 97` can be 25.
fn index(c: char) -> { i: u32 | i < 25 } {
    if 'a' <= c && c <= 'z' { c as u32 - 'a' as u32 } else { 0 }
}

fn main() {
    let _ = index('b');
}
// error: can't prove `i < 25` for the result of `index`
