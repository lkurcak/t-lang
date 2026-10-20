// Each effect that's said but can't happen is a warning of its own.
fn one() -> int with {alloc, panic} { 1 + 1 }

fn main() with {alloc, panic} {
    print(one()); // expect: 2
}
// warning: `one` says `with alloc`, but it can't allocate
