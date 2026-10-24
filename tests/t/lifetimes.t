// Lifetimes: how long a reference is good for. Signatures say how the
// references they take and return are related; elision fills in the
// obvious ones.

#[derive(Discard)]
struct Pair { a: i64, b: i64 }

// Elided: one lifetime in, so the result has it.
fn first(p: &Pair) -> &i64 { &p.a }

// Written: the result lives as long as both arguments do.
fn longer<'a>(x: &'a Pair, y: &'a Pair) -> &'a Pair with panic {
    if x.a + x.b >= y.a + y.b { x } else { y }
}

// The result borrows from `x` only, so `y` can be anything.
fn left<'a, 'b>(x: &'a i64, y: &'b i64, seen: &mut i64) -> &'a i64 {
    *seen = *y;
    x
}

// A struct holding a reference has a lifetime parameter.
#[derive(Discard)]
struct View<'a> { pair: &'a Pair, which: bool }

fn get(v: View) -> i64 {
    if v.which { v.pair.a } else { v.pair.b }
}

// `'b: 'a` says `'b` lasts at least as long as `'a`.
fn shorter<'a, 'b: 'a>(x: &'a i64, y: &'b i64) -> &'a i64 {
    if *x < *y { x } else { y }
}

// A reference to a reference: the inner one outlives the outer.
fn deref<'a, 'b>(r: &'a &'b i64) -> &'b i64 { *r }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let p = Pair { a: 1, b: 2 };
    let q = Pair { a: 3, b: 4 };
    out.line(&(*first(&p)))?;                      // expect: 1
    out.line(&longer(&p, &q).b)?;                // expect: 4
    let n = 5;
    let mut r = &p.a;
    let mut seen = 0;
    {
        let m = 6;
        r = left(&n, &m, &mut seen);
    }
    out.line(&seen)?;                            // expect: 6
    out.line(&(*r))?;                              // expect: 5
    out.line(&get(View { pair: &q, which: true }))?; // expect: 3
    out.line(&(*shorter(&n, &p.a)))?;              // expect: 1
    let rn = &n;
    out.line(&(*deref(&rn)))?;                     // expect: 5
    Result::Ok(())
}
