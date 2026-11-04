// What a mutex lends lives only as long as the closure runs: it can't get
// out, where it could be used after the mutex is unlocked.
use core::sync::Mutex;

fn main() {
    let m = Mutex::new(1u32);
    let r = m.lock(|x: &mut u32| x);
    // error: this function isn't general enough
    *r = 2;
}
