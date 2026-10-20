// Function types only gain effects: passing one that may panic where one
// that can't is wanted is no more allowed than storing it.
fn call(f: fn() -> int) -> int { f() }

fn boom() -> int with panic { panic() }

fn main() with panic {
    print(call(boom));
}
// error: expected `fn() -> int`, found `fn() -> int with panic`
