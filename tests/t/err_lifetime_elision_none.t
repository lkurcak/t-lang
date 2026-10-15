fn make() -> &int { &1 }

fn main() {}
// error: missing lifetime: this returns a reference, but there's nothing it could borrow from
