// Two values of an enum are the same only if they're the same variant:
// variants with no fields are told apart by which they are
// (docs/85-values-in-predicates.md).
#[derive(Copy, Discard)]
enum Light { Red, Green }

fn set_red(a: &mut Light, b: &Light) ensures *a == *b { *a = Light::Red; }

fn main() {
    let mut x = Light::Green;
    set_red(&mut x, &Light::Green);
}
// error: can't prove `*a == *b` for what `set_red` ensures
