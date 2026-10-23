// Function types only gain effects: passing one that may panic where one
// that can't is wanted is no more allowed than storing it.
fn call(f: fn() -> i64) -> i64 { f() }

fn boom() -> i64 with panic { panic("stop") }

fn main() with panic {
    print(call(boom));
}
// error: expected `fn() -> i64`, found `fn() -> i64 with panic`
