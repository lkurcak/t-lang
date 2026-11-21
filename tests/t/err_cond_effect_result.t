// A condition is of what the function is given, at the call: not of what
// it gives back, which there's none of if it panics.
fn f(x: u8) -> { r: u8 | r > 0 } with panic if r > 3 { x + 1 }

fn main() {}
// error: cannot find value `r` in this refinement
