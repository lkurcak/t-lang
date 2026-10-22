// An effect parameter stands for whatever the caller's closure does, so
// calling it has that effect, which the function must say.
fn apply<e, F: Fn(i64) -> i64 with e + Discard>(f: F) -> i64 { f(1) }

fn main() with panic {
    print(apply(|x| x / 0));
}
// error: `f` may have the effects `e`, but `apply` doesn't say `with e`
