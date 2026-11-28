// `inner`'s `F::e` is decided where it's called, by what `outer` knows of
// its `G`: `{alloc}`, for every `G` `outer` is called with. So the copy of
// `inner` for `main`'s closure, which does nothing, still knows `h` may
// allocate (lemma O2 of docs/104-open-fn-bounds.md).
use alloc::boxed::Box;

fn allocates() with alloc {
    let _b = Box::new(1u8);
}

fn inner<F: Fn() + Discard>(_f: F, h: fn() with F::e) { h() }

fn outer<G: Fn() with alloc + Discard>(g: G) { inner(g, allocates) }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    outer(|| {});
    out.line(&1u8)?; // expect: 1
    Ok(())
}
