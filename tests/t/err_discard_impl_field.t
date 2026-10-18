// Nor can a written impl say so.
struct Fd { n: int }

struct Holder { fd: Box<Fd> }

impl Discard for Holder {}

fn main() {}
// error: `Holder` can't be `Discard`: its field `fd` holds a `Box<Fd>`, which isn't
