// Imported under a lowercase name, a variant couldn't be matched by it: the
// name in a pattern would bind a new variable (step 45).
use Shape::Empty as empty;
// error: a variant's name starts with an uppercase letter, so it can't be brought in as `empty`

#[derive(Discard)]
enum Shape {
    Empty,
}

fn main() {}
