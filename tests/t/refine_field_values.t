// What's known of a struct's fields goes with its value: from a literal,
// through a copy, and stays with the copy when the original changes.
#[derive(Copy, Discard)]
struct P { x: int }

fn main() with {alloc, panic} {
    let mut p = P { x: 4 };
    let q = p;
    p.x = 0;
    print(10 / q.x); // expect: 2
    p = q;
    print(12 / p.x); // expect: 3
    let r = P { x: 2 };
    let mut b = Box::new(r);
    print(10 / b.x); // expect: 5
    b.x = 5;
    print(10 / b.x); // expect: 2
}
