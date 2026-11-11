// Nor through an alias: what it stands for is what's written.
type Big = { x: u32 | x > 1 };

extern fn halve(x: Big) -> u32 { x / 2 }
// error: an exported `extern fn` takes no refinement

fn main() {}
