// What a const is isn't known while bodies are checked, so it covers no
// value: a `match` of consts needs a `_`, even where they'd cover all.
const YES: bool = true;
const NO: bool = false;

fn f(b: bool) -> u8 {
    match b {
        YES => 1,
        NO => 0,
    }
}

fn main() {
    let _ = f(true);
}
// error: non-exhaustive `match`
