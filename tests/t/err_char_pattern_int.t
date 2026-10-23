// A character isn't a number, in a pattern either.
fn f(c: char) -> bool {
    match c {
        97 => true,
        _ => false,
    }
}

fn main() {}
// error: expected `char`, found `{integer}`
