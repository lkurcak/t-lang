// A variant's name starts with an uppercase letter (step 45): a lowercase
// one in a pattern would be a new variable instead.
#[derive(Discard)]
enum Shape {
    empty,
// error: a variant's name starts with an uppercase letter
    Dot(i64),
}

fn main() {}
