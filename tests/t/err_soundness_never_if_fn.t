// The branches of an `if` that give functions must give the same type.
struct Fd { n: i64 }

fn die() -> ! {
    loop {}
}

fn one() -> i64 { 1 }

fn main() {
    let f = if true { one } else { die };
    let _fd = Fd { n: 1 };
    f();
}
// error: `if` and `else` have different types: `fn() -> i64` and `fn() -> !`
