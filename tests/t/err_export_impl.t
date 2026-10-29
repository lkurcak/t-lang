// An impl isn't an item that can be seen or not: a trait impl is wherever
// its trait and type are, and an inherent impl's methods say for themselves.
#[derive(Discard)]
export struct S {}

export impl S {}

fn main() {}
// error: an `impl` can't be `export`
