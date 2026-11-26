// The const's type is what the value's must be.
const N: u8 = 3;

fn f(x: i32) -> u8 {
    match x {
        N => 1,
        _ => 0,
    }
}

fn main() {
    let _ = f(3);
}
// error: `N` is a const of type `u8`, but the value matched is `i32`
