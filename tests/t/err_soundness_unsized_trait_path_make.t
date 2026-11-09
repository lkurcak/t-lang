// A path to a trait's method may have `Self` unsized, as `s.fmt(w)` may.
// Here what `Make::make()` gives is a `Self` that's worked out to be `D`
// only after: a value with no size, which the check of every value's type,
// once they're all known, finds.
trait Make { fn make() -> Self; }

fn same<unsized T>(_a: &T, _b: &T) {}

fn f<unsized D: Make + Discard>(x: &D) {
    let r = &Make::make();
    same(r, x);
}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
