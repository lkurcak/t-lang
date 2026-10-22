// Running out of memory aborts, so allocating may panic too.
fn boxed(n: i64) -> Box<i64> with alloc { Box::new(n) }

fn main() with {alloc, panic} {
    print(*boxed(1));
}
// error: `Box::new` may panic, but `boxed` doesn't say `with panic`
