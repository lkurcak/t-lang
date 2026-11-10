// A derived `Discard` or `Copy` of a type whose parameter has an `Fn`
// bound needs only more of the same of its fields: the bound is the
// type's, which holds wherever the type is made (lemma D1 of
// docs/17-retrospective-1.md). So whether a value of it is copied or moved,
// and how a closure holding one is called, don't wait for what its
// closure's effects turn out to be (docs/64-phase-order.md).
#[derive(Discard where F: Discard)]
struct Holder<F: Fn() -> i64> { f: F }

#[derive(Discard, Copy where F: Copy)]
struct Pair<F: Fn(u8) -> u8> { f: F, n: u8 }

fn call<F: Fn() -> i64>(h: &Holder<F>) -> i64 { (h.f)() }

fn use_pair<F: Fn(u8) -> u8 + Copy>(p: Pair<F>) -> u8 { (p.f)(p.n) }

fn twice<G: Fn() -> i64 + Discard>(g: G) -> i64 { g() + g() }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let h = Holder { f: || 21 };
    out.line(&call(&h))?; // expect: 21
    // That `x / 2` can't panic is proved, so the closure can't, so it fits
    // the bound, and the pair is `Copy`: used twice, it's copied.
    let p = Pair { f: |x: u8| x / 2, n: 9 };
    let a = use_pair(p);
    let b = use_pair(p);
    out.line(&(a + b))?; // expect: 8
    // A closure that owns a holder is `Fn`, as the holder is `Discard`.
    let k = move || call(&h);
    out.line(&twice(k))?; // expect: 42
    Ok(())
}
