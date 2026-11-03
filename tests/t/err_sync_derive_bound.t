// A derived `Sync` needs only `Sync` of type parameters: that's all its
// fields can need.
#[derive(Discard where T: Discard, Sync where T: Eq)]
// error: a derived `Sync` can only need `Sync` of type parameters
struct W<T> { t: T }

fn main() {}
