// A handle can't be put where it would never be joined: a static is never
// dropped. Its value is plain data, worked out while compiling, which a
// handle isn't (nor is it `Discard`, which a static's type must be too).
use core::sync::Mutex;
use std::thread::JoinHandle;

static PARKED: Mutex<Option<JoinHandle<'static, u32>>> = Mutex::new(Option::None);
// error: a static can't be of type `Mutex<Option<JoinHandle<'static, u32>>>`

fn main() {}
