// While a thread may read `n`, nothing may change it: the handle holds the
// closure's borrow of `n` until it's joined.
fn main(os: std::Os) with alloc {
    let mut n = 1u32;
    let h = os.threads.spawn(|| core::num::wrapping_add(n, 1));
    n = 5;
// error: cannot assign to `n` while `n` is borrowed
    let _ = core::num::wrapping_add(h.join(), n);
}
