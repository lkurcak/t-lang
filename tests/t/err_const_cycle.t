// A const's value can't depend on itself.
const A: u8 = B + 1;
const B: u8 = A + 1;
// error: the value of const `A` depends on itself

fn main() {
    let _ = A;
}
