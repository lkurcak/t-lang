// A const has no fields to match.
const N: u8 = 3;

fn f(x: u8) -> u8 {
    match x {
        N(y) => y,
        _ => 0,
    }
}

fn main() {
    let _ = f(3);
}
// error: `N` is a const, not a variant: it has no fields
