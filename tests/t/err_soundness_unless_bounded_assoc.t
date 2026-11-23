// Under the bound of an `unless`, a site has the effect if what it calls
// may: here `it.next()`, of `I::e`, which `I`'s bound says is within
// `{panic}`. The walk without the bound counts that `panic`, and so must
// the walk with it (lemma B4 of docs/92-where-a-bound-holds.md; a hole
// until 0.106.2, where it looked for `panic` written out).
trait Small {}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {}

#[derive(Discard)]
struct Down { n: u64 }

impl Iterator for Down {
    type Item = u64;
    effect e = panic;

    fn next(&mut self) -> Option<u64> with panic { panic("down") }
}

fn f<T: Discard, I: Iterator<Item = u64, e = {panic}> + Discard>(t: T, mut it: I) -> u8
with panic unless T: Small
{
    t.discard();
    let _ = it.next();
    it.discard();
    1
}

fn main() with {} {
    let _ = f(A { n: 1 }, Down { n: 3 });
}
// error: `next` may panic where `T: Small`, but `f` says `with panic unless T: Small`
