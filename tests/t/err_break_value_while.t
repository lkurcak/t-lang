fn main() {
    let mut i = 0;
    while i < 10 {
        if i == 5 { break i; }
        i += 1;
    }
}
// error: a `break` with a value only works in a `loop`: a `while` loop can end without a `break`
