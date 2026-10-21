// A `bool` can be refined too, and a `bool` variable remembers what it was
// computed from.
fn needs(b: { b: bool | b }) -> int {
    if b { 1 } else { 0 }
}

fn div(a: int, d: { d: int | d != 0 && d != -1 }) -> int { a / d }

fn main() {
    let x = 5;
    let positive = x > 0;
    if positive {
        print(div(10, x)); // expect: 2
    }
    print(needs(true)); // expect: 1
    print(needs(x == 5)); // expect: 1
}
