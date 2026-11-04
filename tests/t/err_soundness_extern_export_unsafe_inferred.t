// An export's body keeps its own obligations, as `main`'s does: its
// effects are never inferred to be `unsafe`.
extern fn abs(x: i32) -> i32;

extern fn wrapped(x: i32) -> i32 { abs(x) }
// error: `abs` is unsafe, but it isn't in an `unsafe` block

fn main() {}
