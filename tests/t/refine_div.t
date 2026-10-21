// A divisor known to be positive can't be zero, nor -1 under the least
// `int`, so dividing by it can't panic.
fn div(a: int, d: { d: int | d > 0 }) -> int { a / d }

fn main() {
    print(div(10, 3)); // expect: 3
    let n = 7;
    print(div(-14, n)); // expect: -2
    let k = div(100, 9);
    if k > 0 {
        print(div(50, k)); // expect: 4
    }
}
