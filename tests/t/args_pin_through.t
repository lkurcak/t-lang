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

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut a = Game { x: 0 };
    f(&mut a);
    out.line(&a.x)?; // expect: 3422
    let mut b = Game { x: 7 };
    put(&mut b, { b = Game { x: 4 }; 5 });
    out.line(&b.x)?; // expect: 45
    Ok(())
}
