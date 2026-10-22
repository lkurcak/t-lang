// An effect parameter said but never had: `f` is never called.
fn ignore<e, F: Fn() with e + Discard>(f: F) with e {}

fn main() {
    ignore(|| {});
    print(1i64); // expect: 1
}
// warning: unused variable `f`
// warning: `ignore` says `with e`, but nothing in it has the effects `e`
