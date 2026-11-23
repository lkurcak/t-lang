// A trait's default that says `unless` is walked under its bound, where a
// call whose associated effect a bound says is within `{panic}` may panic
// (lemma B4 of docs/92-where-a-bound-holds.md; a hole until 0.106.2).
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

trait Bump {
    fn bump<I: Iterator<Item = u64, e = {panic}> + Discard>(&self, mut it: I) -> u8
    with panic unless Self: Small
    {
        let _ = it.next();
        it.discard();
        1
    }
}

impl Bump for A {}

fn main() with {} {
    let a = A { n: 1 };
    let _ = a.bump(Down { n: 3 });
    a.discard();
}
// error: `next` may panic where `Self: Small`, but `bump` says `with panic unless Self: Small`
