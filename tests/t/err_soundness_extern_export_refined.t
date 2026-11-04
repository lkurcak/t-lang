// C proves nothing of what it passes: an export can't take a refinement,
// or its body would count on what nobody checked.
extern fn halve(x: { x: u32 | x % 2 == 0 }) -> u32 { x / 2 }
// error: an exported `extern fn` takes no refinement

fn main() {}
