// Saying a function may panic when it can't is a promise callers have to
// keep for nothing.
fn one() -> i64 with panic { 1 }

fn main() with panic {
    print(one()); // expect: 1
}
// warning: `one` says `with panic`, but it can't panic
