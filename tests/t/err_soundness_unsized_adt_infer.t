// Nor `str` worked out from a field.
struct Holder<'a, T> { r: &'a T }

fn main() {
    let _h = Holder { r: "abc" };
}
// error: a type parameter that isn't `unsized` can't stand for `str`
