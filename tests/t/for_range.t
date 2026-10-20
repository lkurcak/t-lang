// `for` over a range: `a..b` counts from `a` up to, not including, `b`. A
// range whose start isn't before its end is empty.

fn sum_to(n: int) -> int {
    let mut s = 0;
    for i in 0..n {
        s = s + i;
    }
    s
}

fn main() {
    print(sum_to(5)); // expect: 10
    print(sum_to(0)); // expect: 0
    print(sum_to(-3)); // expect: 0

    // `break` and `continue`, and labels.
    let mut found = 0;
    for i in 1..100 {
        if i % 7 != 0 {
            continue;
        }
        found = i;
        break;
    }
    print(found); // expect: 7
    let mut pairs = 0;
    'outer: for i in 0..10 {
        for j in 0..10 {
            if i * j > 6 {
                continue 'outer;
            }
            if i == 5 {
                break 'outer;
            }
            pairs = pairs + 1;
        }
    }
    print(pairs); // expect: 26

    // Precedence: arithmetic before `..`.
    let n = 2;
    let mut c = 0;
    for _ in n - 1..n * 3 {
        c = c + 1;
    }
    print(c); // expect: 5

    // It doesn't overflow at the very end of `int`.
    let big = 9223372036854775807;
    let mut last = 0;
    for i in big - 2..big {
        last = i;
    }
    print(last - big); // expect: -1

    // A range is a value, and its ends can be read.
    let r = 3..8;
    print(r.start() + r.end()); // expect: 11
    let mut r = r;
    print(r.next().unwrap_or(0)); // expect: 3
    print(r.start()); // expect: 4

    // A `for` is `()`: one can end a block.
    let f = || for i in 0..3 { print(i * 10); };
    f();
    // expect: 0
    // expect: 10
    // expect: 20
}
