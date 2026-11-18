// What the later arguments may still do: write through the reference
// (`g.x = 3`), reborrow it (`at(g, ..)`, `&mut *g`), and give a place that
// isn't through a reference a new value, which is the same place after
// (lemma A5 of docs/83).
#[derive(Discard)]
struct Game { x: u32 }

fn put(g: &mut Game, v: u32) { g.x = g.x * 10 + v; }

fn at(g: &mut Game) -> u32 { g.x }

fn f(g: &mut Game) {
    put(g, { g.x = 3; 1 });
    put(g, at(g));
    put(g, { let h = &mut *g; h.x = h.x + 1; 2 });
}

fn small(v: u32) -> Result<u32, u32> {
    if v < 10 { Ok(v) } else { Err(v) }
}

/// A `?` in a later argument may leave the function, dropping `g`: the
/// call borrows nothing then.
fn tried(g: &mut Game) -> Result<(), u32> {
    put(g, small(g.x)?);
    Ok(())
}

fn id(g: &mut Game) -> &mut Game { g }

fn twice(v: u32) -> u32 { v * 2 }

/// A call in an argument pins what its own borrows go through, until it
/// takes them; the `?` after it, which drops what it borrowed through,
/// is the outer call's argument.
fn nested(g: &mut Game) -> Result<u32, u32> {
    Ok(twice(small(at(id(g)))?))
}

/// A `break` in a later argument leaves the call: `g` may change after.
fn leave(a: &mut Game, b: &mut Game) {
    let mut g = a;
    loop {
        put(g, { if g.x > 100 { break; } 1 });
        break;
    }
    g = b;
    put(g, 7);
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut a = Game { x: 0 };
    f(&mut a);
    out.line(&a.x)?; // expect: 3422
    let mut b = Game { x: 7 };
    put(&mut b, { b = Game { x: 4 }; 5 });
    out.line(&b.x)?; // expect: 45
    let mut c = Game { x: 200 };
    let mut d = Game { x: 0 };
    leave(&mut c, &mut d);
    out.line(&c.x)?; // expect: 200
    out.line(&d.x)?; // expect: 7
    let mut e = Game { x: 4 };
    let _ = tried(&mut e);
    let _ = tried(&mut e);
    out.line(&e.x)?; // expect: 44
    let mut f = Game { x: 3 };
    out.line(&nested(&mut f).unwrap_or(0))?; // expect: 6
    Ok(())
}
