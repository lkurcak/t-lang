// The effects are `panic` and `alloc`, and effect parameters in scope.
fn f() with io {}

fn main() { f() }
// error: there's no effect `io`
