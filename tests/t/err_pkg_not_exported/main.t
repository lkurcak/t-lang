// `pub` is seen in the package; another package sees only what's `export`.
fn main() {
    let _a = lib::outside();
    let _b = lib::inside(); // error: isn't exported by `lib`
}
