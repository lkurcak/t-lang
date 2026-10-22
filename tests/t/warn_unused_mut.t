// A variable declared `mut` that's never changed. The fix removes the `mut`.

fn count(mut n: i64) -> i64 with panic { n + 1 }

fn main() with panic {
    let mut x = 1;
    let mut y = 2;
    y += x;
    print(y);           // expect: 3
    print(count(x));    // expect: 2
    let f = |mut z: i64| z * 2;
    print(f(2));        // expect: 4
}
// warning: variable `n` doesn't need to be mutable
// warning: variable `x` doesn't need to be mutable
// warning: variable `z` doesn't need to be mutable
