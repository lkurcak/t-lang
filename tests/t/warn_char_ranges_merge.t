// Ranges of characters that touch can be merged, and the fix writes
// characters.
fn f(c: char) -> bool {
    match c {
        'a'..='m' | 'n'..='z' => true,
        _ => false,
    }
}

fn main() {
    print_bool(f('q')); // expect: true
}
// warning: `'a'..='m'` and `'n'..='z'` can be merged into `'a'..='z'`
