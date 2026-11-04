// What a C function gives back is only what its C type says: an import's
// result can't be refined, or the checker would trust C to prove it.
extern fn positive() -> { x: i32 | x > 0 };
// error: an imported `extern fn` gives back no refinement

fn main() {}
