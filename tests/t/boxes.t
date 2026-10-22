// Boxes: values on the heap, freed when their owner goes out of scope.
// The interpreter checks that each is freed exactly once.

#[derive(Discard)]
struct Big { a: i64, b: i64 }

fn make(n: i64) -> Box<Big> with {panic, alloc} { Box::new(Big { a: n, b: n * 2 }) }

fn total(b: &Box<Big>) -> i64 with panic { b.a + b.b }

fn main() with {alloc, panic} {
    let b = make(3);
    print(total(&b));           // expect: 9
    print(b.b);                 // expect: 6
    let c = b;                  // moved: `c` owns it now
    let inner = *c;             // moved out of the box
    print(inner.a);             // expect: 3
    let mut n = Box::new(1i64);
    *n += 41;
    print(*n);                  // expect: 42
    n = Box::new(7);            // the old box is freed here
    print(*n);                  // expect: 7
    let nested = Box::new(Box::new(5i64));
    print(**nested);            // expect: 5
}
