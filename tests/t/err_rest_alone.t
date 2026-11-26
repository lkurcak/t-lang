// `..` stands for values of a tuple, variant or array: it isn't a pattern
// of its own.
fn main() {
    let .. = 5u8;
}
// error: `..`
