// Boxes: values on the heap, freed when their owner goes out of scope.
// The interpreter checks that each is freed exactly once.

use alloc::boxed::Box;

#[derive(Discard)]
struct Big { a: i64, b: i64 }

fn make(n: i64) -> Box<Big> with {panic, alloc} { Box::new(Big { a: n, b: n * 2 }) }

fn total(b: &Box<Big>) -> i64 with panic { b.a + b.b }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut stdout = os.stdout;
    let b = make(3);
    stdout.line(&total(&b))?;           // expect: 9
    stdout.line(&b.b)?;                 // expect: 6
    let c = b;                  // moved: `c` owns it now
    let inner = *c;             // moved out of the box
    stdout.line(&inner.a)?;             // expect: 3
    let mut n = Box::new(1i64);
    *n += 41;
    stdout.line(&(*n))?;                  // expect: 42
    n = Box::new(7);            // the old box is freed here
    stdout.line(&(*n))?;                  // expect: 7
    let nested = Box::new(Box::new(5i64));
    stdout.line(&(**nested))?;            // expect: 5
    Result::Ok(())
}
