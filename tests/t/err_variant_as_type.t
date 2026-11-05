// A variant isn't a type.
use Shape::Dot;

#[derive(Discard)]
enum Shape { Dot(i64) }

fn f(d: Dot) {}
// error: unknown type `Dot`

fn main() {}
