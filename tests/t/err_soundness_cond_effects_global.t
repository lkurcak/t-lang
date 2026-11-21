// A static used where the condition doesn't hold is one the function
// doesn't say.
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn step(counted: bool) -> u32 with global if counted {
    match counted {
        true => 0,
        false => COUNT.fetch_add(1),
    }
}

fn main() with {} {
    let _v = step(false);
}
// error: `COUNT` is used when `counted` doesn't hold, but `step` says `with global if counted`
