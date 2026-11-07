// Only dropping a `Raw` puts the terminal back: leaving raw mode by hand
// would leave a `Raw` that thinks it's still raw.
fn main(os: std::Os) {
    let mut input = os.stdin;
    input.os_raw_leave(); // error: private
}
