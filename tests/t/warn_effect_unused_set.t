// Each effect that's said but can't happen is a warning of its own.
fn one(n: int) -> int with {alloc, panic} { n + 1 }

fn main() with {alloc, panic} {
    print(one(1)); // expect: 2
}
// warning: `one` says `with alloc`, but it can't allocate
