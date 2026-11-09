// `Ord` needs `Eq`: a derived one too.
#[derive(Copy, Discard, Ord)]
struct V { n: u8 }

fn main() {}
// error: `V` doesn't implement `Eq`, which `Ord` needs, as it's `trait Ord: Eq`
