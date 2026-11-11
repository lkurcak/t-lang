// Nor through an alias: what it stands for is what's written.
type Positive = { x: i32 | x > 0 };

extern fn positive() -> Positive;
// error: an imported `extern fn` gives back no refinement

fn main() {}
