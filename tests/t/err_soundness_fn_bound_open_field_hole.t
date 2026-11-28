// Were a field allowed to name `F::e`, `outer`, which knows `G` only as
// `G: Fn() with alloc`, could store an allocating function as a `fn() with
// G::e`; `main`, whose `G` is a closure that does nothing, would read it
// as a `fn()` and call it as one (lemma O4 of docs/104-open-fn-bounds.md).
use alloc::boxed::Box;

fn allocates() with alloc {
    let _b = Box::new(1u8);
}

#[derive(Discard where F: Discard)]
struct S<F: Fn()> { f: F, h: fn() with F::e }

fn outer<G: Fn() with alloc + Discard>(g: G) -> S<G> { S { f: g, h: allocates } }

fn main() {
    let s = outer(|| {});
    (s.h)();
}
// error: a field can't have what calls of `F` do, `F::e`, in its type
