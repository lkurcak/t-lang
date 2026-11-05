use Shape::Dot::X;
// error: `Shape` isn't a module

#[derive(Discard)]
enum Shape { Dot(i64) }

fn main() {}
