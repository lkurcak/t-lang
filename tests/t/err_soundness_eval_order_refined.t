// What an earlier operand gave is what it was then: 5, not the 7 a later
// one makes its variable (docs/91-evaluation-order.md).
fn first(a: u8, _b: u8) -> { r: u8 | r == a } { a }

fn need7(x: { x: u8 | x == 7 }) -> u8 { x }

fn main() {
    let mut v = 5u8;
    let _r = need7(first(v, { v = 7; 7 }));
}
// error: can't prove `x == 7` for argument `x` of `need7`
