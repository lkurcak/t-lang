// Lifetimes: how long a reference is good for. Signatures say how the
// references they take and return are related; elision fills in the
// obvious ones.

struct Pair { a: int, b: int }

// Elided: one lifetime in, so the result has it.
fn first(p: &Pair) -> &int { &p.a }

// Written: the result lives as long as both arguments do.
fn longer<'a>(x: &'a Pair, y: &'a Pair) -> &'a Pair {
    if x.a + x.b >= y.a + y.b { x } else { y }
}

// The result borrows from `x` only, so `y` can be anything.
fn left<'a, 'b>(x: &'a int, y: &'b int) -> &'a int {
    print(*y);
    x
}

// A struct holding a reference has a lifetime parameter.
struct View<'a> { pair: &'a Pair, which: bool }

fn get(v: View) -> int {
    if v.which { v.pair.a } else { v.pair.b }
}

// `'b: 'a` says `'b` lasts at least as long as `'a`.
fn shorter<'a, 'b: 'a>(x: &'a int, y: &'b int) -> &'a int {
    if *x < *y { x } else { y }
}

// A reference to a reference: the inner one outlives the outer.
fn deref<'a, 'b>(r: &'a &'b int) -> &'b int { *r }

fn main() {
    let p = Pair { a: 1, b: 2 };
    let q = Pair { a: 3, b: 4 };
    print(*first(&p));                      // expect: 1
    print(longer(&p, &q).b);                // expect: 4
    let n = 5;
    let mut r = &p.a;
    {
        let m = 6;
        r = left(&n, &m);                   // expect: 6
    }
    print(*r);                              // expect: 5
    print(get(View { pair: &q, which: true })); // expect: 3
    print(*shorter(&n, &p.a));              // expect: 1
    let rn = &n;
    print(*deref(&rn));                     // expect: 5
}
