// An operator's operand must be of a type it works on, and `!`, of what
// never finishes, isn't one: as for `-panic()`.
fn never() -> bool with panic { !panic() }

fn main() {}
// error: `!` works on `bool` and integers, not on `!`
