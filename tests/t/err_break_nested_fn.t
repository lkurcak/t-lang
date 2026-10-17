fn main() {
    loop {
        fn helper() { break; }
        helper();
    }
}
// error: `break` outside of a loop
