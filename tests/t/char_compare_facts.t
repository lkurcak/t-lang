// `<` of two `char`s compares their numbers, as `c as u32` does, and the
// checker knows it: a guard on characters says what their numbers are.
fn square(file: char, rank: char) -> Option<{ s: u8 | s < 64 }> with {} {
    if file < 'a' || file > 'h' || rank < '1' || rank > '8' {
        return None;
    }
    Some(((rank as u32 - '1' as u32) * 8 + (file as u32 - 'a' as u32)) as u8)
}

fn digit(c: char) -> Option<{ d: u32 | d < 10 }> with {} {
    if '0' <= c && c <= '9' { Some(c as u32 - '0' as u32) } else { None }
}

// Through `Ord`, as generic code compares, the same answers.
fn less<T: Ord>(a: &T, b: &T) -> bool { *a < *b }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&square('e', '4').unwrap_or(99))?; // expect: 28
    out.line(&square('i', '4').unwrap_or(99))?; // expect: 99
    out.line(&square('a', '9').unwrap_or(99))?; // expect: 99
    out.line(&digit('7').unwrap_or(99))?; // expect: 7
    out.line(&digit('x').unwrap_or(99))?; // expect: 99
    out.line(&less(&'a', &'b'))?; // expect: true
    out.line(&('z' < 'é'))?; // expect: true
    out.line(&('é' >= 'z'))?; // expect: true
    out.line(&core::cmp::max('q', 'c'))?; // expect: q
    Ok(())
}
