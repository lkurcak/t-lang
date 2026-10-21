// Proving the operators can't panic leaves nothing that may.
fn add_one(x: { v: int | v < 10 }) -> int with panic { x + 1 }

fn main() with panic {
    print(add_one(1)); // expect: 2
}
// warning: `add_one` says `with panic`, but it can't panic
