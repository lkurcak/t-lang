// `F::e` is `<F as FnOnce>::e`, unless `F` has another trait with an
// associated effect `e`: then it could be either.
fn f<F: Iterator + Fn() + Discard>(g: F) with F::e { g() }

fn main() {}
// error: `F::e` could be `<F as Iterator>::e` or `<F as FnOnce>::e`; write which
