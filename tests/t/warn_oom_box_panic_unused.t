// Allocating doesn't panic, so `panic` here is said for nothing.

use alloc::boxed::Box;

fn boxed(n: i64) -> Box<i64> with {alloc, panic} { Box::new(n) }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    out.line(&(*boxed(1)))?; // expect: 1
    Result::Ok(())
}
// warning: `boxed` says `with panic`, but it can't panic
