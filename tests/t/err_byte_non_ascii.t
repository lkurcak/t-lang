// A byte literal holds bytes, written as ASCII or `\x..`: `é` is two.
fn main() {
    print(b"café".len());
}
// error: a byte literal holds only ASCII characters; write other bytes as `\x..`
