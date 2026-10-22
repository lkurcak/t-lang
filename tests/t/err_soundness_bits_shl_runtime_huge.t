// Far past the width: the interpreter mustn't wrap the amount.
fn f(x: u64, n: u32) -> u64 with panic { x << n }

fn main() with panic {
    print(f(0, 4294967295));    // expect: 0
    print(f(1, 4294967295));
}
// error: runtime error: integer overflow
