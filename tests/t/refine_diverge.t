// A branch that never ends tells what comes after that its condition
// didn't hold.
fn div(a: int, d: { d: int | d > 0 }) -> int { a / d }

fn checked(a: int, d: int) -> int with panic {
    if d <= 0 {
        panic();
    }
    div(a, d)
}

fn main() with panic {
    print(checked(9, 3)); // expect: 3
}
