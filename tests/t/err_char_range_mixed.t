fn f(c: char) -> bool {
    match c {
        'a'..=122 => true,
        _ => false,
    }
}

fn main() {}
// error: a range of characters goes from a character to a character
