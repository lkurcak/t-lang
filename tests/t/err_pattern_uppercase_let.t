// A variable's name can't start with an uppercase letter: in a pattern
// that's a path (step 45), and this one names nothing.
fn main() {
    let Total = 5;
// error: `Total` isn't a variant in scope; a variable's name starts with a lowercase letter
    let _ = Total;
}
