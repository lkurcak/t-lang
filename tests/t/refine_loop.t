// A `let mut` with a refined type keeps its refinement, checked at each
// assignment; the loop's condition holds in its body.
fn count(n: int) -> int {
    let mut i: { v: int | v >= 0 } = 0;
    let mut steps: { v: int | v >= 0 } = 0;
    while i < n {
        i = i + 1;
        steps = i;
    }
    steps
}

fn main() {
    print(count(5)); // expect: 5
    print(count(-3)); // expect: 0
}
