// A const's name isn't a pattern yet: in a pattern `N` is a path (step
// 45), never a new variable, and what a const there would mean isn't
// decided.
const N: u8 = 3;

fn is_n(x: u8) -> bool {
    match x {
        N => true,
// error: `N` is a const: a pattern can't name one yet
        _ => false,
    }
}

fn main() {
    let _ = is_n(3);
}
