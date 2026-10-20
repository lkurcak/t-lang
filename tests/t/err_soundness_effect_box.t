// `Box::new` allocates, and aborts if memory has run out.
fn boxed(n: int) -> Box<int> with panic { Box::new(n) }

fn main() with {alloc, panic} {
    print(*boxed(1));
}
// error: `Box::new` may allocate, but `boxed` doesn't say `with alloc`
