fn digit(c: u8) -> Option<{ d: u8 | d < 10 }> {
    if c >= 48 { Some(c - 48) } else { None }
}

fn main() {}
// error: can't prove `d < 10` for the result of `digit`
