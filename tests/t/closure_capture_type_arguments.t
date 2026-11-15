// A closure changing a place in a tuple's element, or in a generic
// struct's field, captures the element or the field whole (K2a of
// docs/79-what-a-capture-splits.md): what it writes is checked against
// the element's refinement, and what's next to it is apart.
#[derive(Discard)]
struct Q { x: u32, y: u32 }

#[derive(Discard)]
struct Pair<T> { a: T, n: u32 }

#[derive(Discard)]
struct W<U> { u: U, k: u32 }

fn need(q: &{ v: Q | v.x <= v.y }) -> u32 { q.y - q.x }

fn make() -> ({ v: Q | v.x <= v.y }, u32) { (Q { x: 3, y: 10 }, 0) }

fn nested() -> Pair<W<{ v: Q | v.x <= v.y }>> {
    Pair { a: W { u: Q { x: 3, y: 10 }, k: 0 }, n: 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut t = make();
    // `x` can be 0 whatever `y` is.
    let mut c = || t.0.x = 0;
    t.1 = 7;
    c();
    out.line(&f"{need(&t.0)} {t.1}")?; // expect: 10 7
    // At the first type argument's place: `w.a`, as `T` could be a `W`
    // with a refinement of its own.
    let mut w = nested();
    let mut d = || w.a.u.y = 4294967295;
    w.n = 2;
    d();
    out.line(&f"{w.a.u.y - w.a.u.x} {w.n}")?; // expect: 4294967292 2
    // Reading goes as far as it names.
    let r = || t.0.y;
    t.1 = 8;
    out.line(&f"{r()} {t.1}")?; // expect: 10 8
    // In a closure's parameter: captured whole, and apart from the rest.
    let mut log: u32 = 0;
    let mut f = |mut q: Q| {
        let mut e = || q.x = 2;
        log = 3;
        e();
        q.x
    };
    out.line(&f"{f(Q { x: 1, y: 2 })}")?; // expect: 2
    out.line(&f"{log}") // expect: 3
}
