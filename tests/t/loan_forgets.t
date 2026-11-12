// A closure's capture, or a pattern's `&mut` binding, is a loan: what's
// known of the place is forgotten where it's made, and nothing can read
// or write the place while it lives. Before it, and after it, the place
// is known as any other (docs/69-loans.md).

/// What's known of `x` before the closure is made holds of what was read.
fn before() -> { v: i64 | v == 5 } {
    let mut x: i64 = 5;
    let y = x;
    let mut inc = || x = x + 1;
    inc();
    y
}

/// Once the closure is gone, an assignment is known again.
fn after() -> { v: i64 | v == 5 } {
    let mut x: i64 = 0;
    let mut inc = || x = x + 1;
    inc();
    x = 5;
    x
}

/// A closure borrowing `d` shared runs only while `d` can't change: it
/// sees what `d` is where it's made.
fn apply<F: FnMut() -> i64 + Discard>(mut f: F) -> i64 { f() }

fn frozen() -> i64 with {} {
    let mut d: i64 = 2;
    let c = || 10 / d;
    let r = apply(c);
    d = 0;
    r + d
}

/// A pattern's `&mut` binding, and then the place written whole.
fn bound(o: &mut Option<i64>) -> { v: i64 | v == 7 } {
    match o {
        Some(n) => *n = 1,
        None => {}
    }
    *o = Some(7);
    match *o {
        Some(n) => n,
        None => 7,
    }
}

#[derive(Copy, Discard)]
struct S { len: u64, b: u64 }

impl S {
    #[measure]
    fn len(&self) -> u64 { self.len }
}

/// What a closure may change can be put back after it.
fn restore(s: &mut S) with panic ensures s.len() == old(s.len()) {
    let l = s.len;
    let mut f = || s.b = s.b + 1;
    f();
    s.len = l;
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&before())?; // expect: 5
    out.line(&after())?; // expect: 5
    out.line(&frozen())?; // expect: 5
    let mut o = Some(3);
    out.line(&bound(&mut o))?; // expect: 7
    let mut s = S { len: 4, b: 0 };
    restore(&mut s);
    out.line(&s.len)?; // expect: 4
    out.line(&s.b)?; // expect: 1
    Ok(())
}
