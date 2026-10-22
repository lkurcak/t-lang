// `if` is an expression; `else if` chains.

fn sign(n: i64) -> i64 {
    if n < 0 { -1 } else if n == 0 { 0 } else { 1 }
}

fn main() {
    print(sign(-7));    // expect: -1
    print(sign(0));     // expect: 0
    print(sign(42));    // expect: 1
    if sign(5) == 1 {
        print(100i64);     // expect: 100
    }
    if false { print(999i64) }
}
