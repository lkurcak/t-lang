// References: `&x` borrows `x`, `*r` is what `r` points to, and a field of
// a reference is a field of what it points to.

#[derive(Copy)]
struct Point { x: int, y: int }

fn sum(p: &Point) -> int { p.x + p.y }

fn first(p: &Point) -> &int { &p.x }

fn larger<'a>(a: &'a Point, b: &'a Point) -> &'a Point {
    if sum(a) >= sum(b) { a } else { b }
}

fn main() {
    let p = Point { x: 1, y: 2 };
    let q = Point { x: 5, y: 5 };
    let r = &p;
    print(sum(r));              // expect: 3
    print(*first(&p));          // expect: 1
    print(larger(&p, &q).x);    // expect: 5
    let rr = &r;
    print(rr.y);                // expect: 2
    print_bool(**rr == p);      // expect: true
    print_bool(&p == &q);       // expect: false
}
