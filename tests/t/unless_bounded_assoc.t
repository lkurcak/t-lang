// An associated effect a function says beside an `unless` is its own,
// whatever the bound: a call of `it.next()`, of `I::e`, is within `{I::e,
// panic unless T: Small}` with the bound too, though `I`'s bound says
// `I::e` may be `panic`. A call has `I::e` where it's called, which is
// `panic` of `Down` (docs/92-where-a-bound-holds.md). And an `unsafe`
// block keeps its obligations with the bound as without it.
trait Small {}

#[derive(Discard)]
struct A { n: u8 }

impl Small for A {}

#[derive(Discard)]
struct Down { n: u64 }

impl Iterator for Down {
    type Item = u64;
    effect e = panic;

    fn next(&mut self) -> Option<u64> with panic {
        if self.n == 0 {
            return None;
        }
        self.n = self.n - 1;
        Some(self.n)
    }
}

/// Unsafe to call, for no reason but to be.
#[allow(unused_effects)]
fn poke() -> u64 with unsafe { 0 }

fn first<T: Discard, I: Iterator<Item = u64, e = {panic}> + Discard>(t: T, mut it: I) -> u64
with {I::e, panic unless T: Small}
{
    t.discard();
    let n = match it.next() {
        Some(n) => n,
        None => 0,
    };
    it.discard();
    // SAFETY: `poke` asks nothing.
    let _k = unsafe { poke() };
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&first(A { n: 1 }, Down { n: 3 }))?; // expect: 2
    Ok(())
}
