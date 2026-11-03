// An exported struct with private fields says what its `Send` needs, as
// its `Discard` does: its fields aren't part of its package's interface.
#[derive(Discard, Send)]
// error: `Holder` is exported with fields that aren't, so its `Send` must say what it needs: `where T: Sync`
export struct Holder<'a, T> { r: &'a T }

fn main() {}
