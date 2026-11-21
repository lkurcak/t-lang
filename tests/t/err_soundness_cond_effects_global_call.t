// A call has a static's use where its condition isn't ruled out.
use core::sync::Atomic;

static COUNT: Atomic<u32> = Atomic::new(0);

fn step(counted: bool) -> u32 with global if counted {
    match counted {
        true => COUNT.fetch_add(1),
        false => 0,
    }
}

fn quiet(c: bool) -> u32 with {} { step(c) }

fn main() with {} {
    let _v = quiet(true);
}
// error: `step` may use a static, but `quiet` doesn't say `with global`
