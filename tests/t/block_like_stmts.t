// An `if`, `match` or block at the start of a statement is the whole
// statement, like in Rust.

fn f(x: i64) -> i64 with panic {
    if x > 100 { print(x) }   // a statement of type `()`
    -x                        // the block's value: not `(if ..) - x`
}

fn g(x: i64) -> i64 with panic {
    // Parenthesize to use one in a larger expression.
    (if x > 0 { 10 } else { 20 }) - x
}

fn h(b: bool) -> i64 {
    match b { true => print(1i64), false => print(0i64) }
    { print(2i64) }
    if b { 3 } else { 4 }     // the block's value, so any type is fine
}

fn main() with panic {
    print(f(5));              // expect: -5
    print(g(3));              // expect: 7
    print(h(true));           // expect: 1
                              // expect: 2
                              // expect: 3
    if true { 1i64 } else { 2 }; // with `;` the value is discarded, any type
}
