// An arena of values that aren't `Discard` isn't either: dropping it
// would throw them away.
use alloc::arena::Arena;

struct Token {}

fn main() with alloc {
    let a = Arena::new();
    let _ = a.alloc(Token {});
}
// error: isn't `Discard`
