// References: `&x` borrows `x`, `*r` is what `r` points to, and a field of
// a reference is a field of what it points to.

#[derive(Discard, Copy, Eq)]
struct Point { x: i64, y: i64 }

fn sum(p: &Point) -> i64 with panic { p.x + p.y }

fn first(p: &Point) -> &i64 { &p.x }

fn larger<'a>(a: &'a Point, b: &'a Point) -> &'a Point with panic {
    if sum(a) >= sum(b) { a } else { b }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let p = Point { x: 1, y: 2 };
    let q = Point { x: 5, y: 5 };
    let r = &p;
    out.line(&sum(r))?;              // expect: 3
    out.line(&(*first(&p)))?;          // expect: 1
    out.line(&larger(&p, &q).x)?;    // expect: 5
    let rr = &r;
    out.line(&rr.y)?;                // expect: 2
    out.line(&(**rr == p))?;      // expect: true
    out.line(&(&p == &q))?;       // expect: false
    Result::Ok(())
}
