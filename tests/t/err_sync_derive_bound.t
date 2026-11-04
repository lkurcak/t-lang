// A derived `Sync` needs only `Send` or `Sync` of type parameters: that's all
// its fields can need (`Send` of what a mutex in one holds).
#[derive(Discard where T: Discard, Sync where T: Eq)]
// error: a derived `Sync` can only need `Send` or `Sync` of type parameters
struct W<T> { t: T }

fn main() {}
