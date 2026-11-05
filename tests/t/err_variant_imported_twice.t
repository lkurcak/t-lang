use Light::Red;
use Color::Red;
// error: `Red` is imported twice

#[derive(Discard)]
enum Light { Red }

#[derive(Discard)]
enum Color { Red }

fn main() {}
