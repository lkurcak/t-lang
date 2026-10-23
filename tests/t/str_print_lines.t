// `print_str` prints one line per call; a string with a newline in it
// prints two.
fn main() {
    print_str("one\ntwo"); // expect: one
    // expect: two
    print_str(""); // expect: 
}
