// A string pattern compares bytes with no C library to do it.
fn code(word: &str) -> u8 {
    match word {
        "" => 0,
        "help" | "h" => 1,
        _ => 2,
    }
}

fn main() with panic {
    assert(code("h") == 1);
    assert(code("help") == 1);
    assert(code("hel") == 2);
    assert(code("") == 0);
}
