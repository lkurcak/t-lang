// `-x` of a `u8` panics unless it's 0.
fn neg(x: u8) -> u8 with panic { -x }

fn main() with panic {
    print(neg(0));  // expect: 0
    print(neg(1));
}
// error: runtime error: integer overflow
