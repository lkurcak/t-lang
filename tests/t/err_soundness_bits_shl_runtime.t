fn f(x: u8) -> u8 with panic { x << 1 }

fn main() with panic {
    print(f(127));  // expect: 254
    print(f(128));
}
// error: runtime error: integer overflow
