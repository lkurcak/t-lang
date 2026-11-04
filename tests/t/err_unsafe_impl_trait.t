// Only `Send` and `Sync` can be promised by `unsafe impl`: they're what
// the checker can't see of a type's fields, as a lock guards them.
#[derive(Discard)]
struct P { x: u32 }

unsafe impl Discard for P {}
// error: only `Send` and `Sync` can be implemented with `unsafe impl`

fn main() {}
