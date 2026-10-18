trait Ord: Eq {}
#[derive(Discard)]
struct P {}
impl Ord for P {}
fn main() {}
// error: `P` doesn't implement `Eq`, which `Ord` needs, as it's `trait Ord: Eq`
