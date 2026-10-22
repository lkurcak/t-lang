// A `bool` can be refined too, and a `bool` variable remembers what it was
// computed from.
fn needs(b: { b: bool | b }) -> i64 {
    if b { 1 } else { 0 }
}

fn div(a: i64, d: { d: i64 | d != 0 && d != -1 }) -> i64 { a / d }

fn main() {
    let x = 5;
    let positive = x > 0;
    if positive {
        print(div(10, x)); // expect: 2
    }
    print(needs(true)); // expect: 1
    print(needs(x == 5)); // expect: 1
}
