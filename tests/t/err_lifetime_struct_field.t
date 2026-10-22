#[derive(Discard)]
struct View { r: &i64 }

fn main() {}
// error: missing lifetime: a reference in a struct or enum needs one
