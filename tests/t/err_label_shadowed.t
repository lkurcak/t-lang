// With both loops labelled `'a`, a `break 'a` in the inner one couldn't
// mean the outer.
fn main() {
    'a: loop {
        'a: loop {
            break 'a;
        }
    }
}
// error: a loop around this one is labelled `'a` already
