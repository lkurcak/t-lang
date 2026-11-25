// A bound an associated effect hangs on, of an array whose length is a
// const item, `[u8; LEN]: G`, isn't decided where it's checked: `LEN` may
// one day be equal to its value (lemma C6 of docs/36-consts.md). In a copy
// it's `[u8; 3]`, and decided: until 0.107.1 the projection was worked out
// before the length was, and left undecided, a crash in monomorphisation
// (retrospective 19).
trait G {}

impl G for [u8; 3] {}

const LEN: usize = 3;

trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct W<T> { x: T }

impl<T: Discard> Go for W<T> {
    effect e = panic unless T: G;

    fn go(&self) -> u8 with Self::e { 1 }
}

#[derive(Discard where T: Discard)]
struct V<T> { x: T }

impl<T: Discard> Go for V<T> {
    effect e = panic unless T: G;

    fn go(&self) -> u8 with Self::e { 2 }
}

fn f(w: &W<[u8; LEN]>) -> u8 with <W<[u8; LEN]> as Go>::e { w.go() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(&W { x: [1u8; LEN] }))?;    // expect: 1
    out.line(&(V { x: [1u8; LEN] }).go())?; // expect: 2
    out.line(&(V { x: [1u8; 4] }).go())?;   // expect: 2
    Ok(())
}
