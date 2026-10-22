// In the head of `if` and `match`, `x {` starts the body, not a struct literal.
// Parenthesize to use a struct literal there.

#[derive(Discard, Eq)]
struct P { x: i64 }

fn main() {
    let x = true;
    if x { print(1i64) }                          // expect: 1
    let p = P { x: 3 };
    if p == (P { x: 3 }) { print(2i64) }          // expect: 2
    match (P { x: 4 }) { P { x } => print(x) } // expect: 4
}
