fn main() {
    // Nothing ever says what `x` is.
    let _f = |x| x;
}
// error: cannot infer the type of `x`; annotate it, e.g. `|x: int|`
