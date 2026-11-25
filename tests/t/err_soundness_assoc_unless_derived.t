// A bound an impl's associated effect hangs on, of a type that derives it,
// holds: `<W<C> as Go>::e` has no `panic`, as monomorphisation and every
// other package work it out. Until 0.107.1 the checker didn't see derived
// impls there, refuted `C: Ord`, and let `f` panic (retrospective 19).
trait Go {
    effect e;

    fn go(&self) -> u8 with Self::e;
}

#[derive(Discard where T: Discard)]
struct W<T> { x: T }

impl<T: Discard> Go for W<T> {
    effect e = panic unless T: Ord;

    fn go(&self) -> u8 with Self::e { 1 }
}

#[derive(Discard, Eq, Ord)]
struct C { n: u8 }

fn f(_w: &W<C>) -> u8 with <W<C> as Go>::e { panic("boom") } // error: `panic` may panic, but `f` doesn't say `with panic`

fn main() with panic {
    let _a = f(&W { x: C { n: 1 } });
}
