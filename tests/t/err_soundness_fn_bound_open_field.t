// A field can't say what calls of its type's parameter do: a function that
// knows `F` only by `F: Fn() with alloc` would read it as `{alloc}`, and a
// value made there could be read where `F` does nothing (lemma O4 of
// docs/104-open-fn-bounds.md).
struct S<F: Fn()> { f: F, h: fn() with F::e }

fn main() {}
// error: a field can't have what calls of `F` do, `F::e`, in its type
