// A const is frozen: a mutex isn't, as what's in it changes through a `&`.
use core::sync::Mutex;

const M: Mutex<u32> = Mutex::new(0);
// error: frozen

fn main() {}
