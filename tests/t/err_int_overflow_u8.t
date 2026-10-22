// An operator panics if its result doesn't fit in its type.
fn add(a: u8, b: u8) -> u8 with panic { a + b }

fn main() with panic {
    print(add(200, 56));
}
// error: runtime error: integer overflow
