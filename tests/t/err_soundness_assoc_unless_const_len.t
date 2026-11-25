// `[u8; LEN]: G` isn't decided where it's checked, though `LEN` has no
// parameter in it: a const item, as a length, may one day be equal to its
// value (lemma C6 of docs/36-consts.md), as it is in a copy, where it holds.
// So `f` is walked again under it, where it can't panic.
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

fn f(_w: &W<[u8; LEN]>) -> u8 with <W<[u8; LEN]> as Go>::e { panic("lie") } // error: `panic` may panic where `[u8; LEN]: G`, but `f` says `with <W<[u8; LEN]> as Go>::e`, which has no `panic` there

fn main() with panic {
    let _r = f(&W { x: [1u8; LEN] });
}
