// A field of a value whose type is refined whole can be written where the
// whole still has its refinement after: of what's behind a `&mut`, a
// tuple's element, a list's, a box's, a variant's.
use alloc::boxed::Box;
use alloc::list::List;

#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

fn gap(s: { s: S | s.a < s.b }) -> u8 with {} { s.b - s.a }

fn step(o: &mut { s: S | s.a < s.b }) {
    if o.a + 1 < o.b {
        o.a = o.a + 1;
    }
}

fn widen(o: &mut Option<{ s: S | s.a < s.b }>) {
    match o {
        Some(s) => s.b = 255,
        None => {},
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut s: { s: S | s.a < s.b } = S { a: 1, b: 4 };
    step(&mut s);
    out.line(&gap(s))?; // expect: 2
    let mut t: (u8, { s: S | s.a < s.b }) = (0, S { a: 1, b: 2 });
    t.1.b = 9;
    t.1.a = 8;
    out.line(&gap(t.1))?; // expect: 1
    let mut l: List<{ s: S | s.a < s.b }> = List::new();
    l.push(S { a: 1, b: 2 });
    l[0].a = 0;
    out.line(&gap(l[0]))?; // expect: 2
    let mut b: Box<{ s: S | s.a < s.b }> = Box::new(S { a: 1, b: 2 });
    b.a = 0;
    out.line(&gap(*b))?; // expect: 2
    let mut o: Option<{ s: S | s.a < s.b }> = Some(S { a: 1, b: 2 });
    widen(&mut o);
    match o {
        Some(s) => out.line(&gap(s))?, // expect: 254
        None => {},
    }
    Ok(())
}
