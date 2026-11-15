// A `&mut` passed on as an argument is reborrowed, `&mut *g`, and that
// borrow waits for the other arguments, as a `&mut *g` written does
// (args-first, docs/06 and docs/81): so `at(g, 1)` reads before `put`
// borrows. Through a variable, a field, or a `*`.
#[derive(Discard)]
struct Game { b: [u8; 4] }

#[derive(Discard)]
struct Holder<'a> { g: &'a mut Game }

fn at(g: &Game, i: { i: usize | i < 4 }) -> u8 { g.b[i] }

fn put(g: &mut Game, i: { i: usize | i < 4 }, v: u8) { g.b[i] = v; }

fn bump(g: &mut Game) -> u8 {
    g.b[3] = g.b[3] % 100 + 1;
    g.b[3]
}

fn by_var(g: &mut Game) {
    put(g, 0, at(g, 1));
    put(g, 2, bump(g));
}

fn by_field(h: &mut Holder) {
    put(h.g, 1, at(h.g, 3));
}

fn by_deref(r: &mut &mut Game) {
    put(*r, 3, at(*r, 0));
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut g = Game { b: [1, 2, 3, 4] };
    by_var(&mut g);
    out.line(&g.b[0])?; // expect: 2
    out.line(&g.b[2])?; // expect: 5
    {
        let mut h = Holder { g: &mut g };
        by_field(&mut h);
    }
    out.line(&g.b[1])?; // expect: 5
    {
        let mut r = &mut g;
        by_deref(&mut r);
    }
    out.line(&g.b[3])?; // expect: 2
    Ok(())
}
