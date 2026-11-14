// `RawBuf::copy_in` copies a slice's values into slots of a buffer that
// hold none, all at once. It's unsafe, as `write` is: the slots must be
// uninitialised.
use alloc::raw::RawBuf;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut b: RawBuf<i64> = RawBuf::new(5);
    let src = [10i64, 20, 30];
    // SAFETY: slots 1..4 are uninitialised.
    unsafe { b.copy_in(1, src.as_slice()) };
    // SAFETY: slots 1..4 hold the copies.
    let s = unsafe { b.slice(1, 4) };
    let (first, last) = (s[0], s[2]);
    // SAFETY: slots 1..4 hold values, which are read out; then every slot
    // is uninitialised.
    unsafe {
        let _ = b.read(1);
        let _ = b.read(2);
        let _ = b.read(3);
        b.free();
    }
    let mut out = os.stdout;
    out.line(&f"{first} {last}")?; // expect: 10 30
    Ok(())
}
