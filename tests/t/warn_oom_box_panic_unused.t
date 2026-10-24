// Allocating doesn't panic, so `panic` here is said for nothing.
fn boxed(n: i64) -> Box<i64> with {alloc, panic} { Box::new(n) }

fn main() with {alloc, panic} {
    print(*boxed(1)); // expect: 1
}
// warning: `boxed` says `with panic`, but it can't panic
