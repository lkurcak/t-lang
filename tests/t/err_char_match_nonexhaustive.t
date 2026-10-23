fn main() {
    let c = 'x';
    match c {
        '\0'..='\u{D7FF}' => {},
    }
}
// error: non-exhaustive `match`: `'\u{e000}'..` not covered
