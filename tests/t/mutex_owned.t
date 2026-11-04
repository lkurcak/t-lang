// A mutex that isn't shared needs no locking: `get_mut` lends what's in it
// through the `&mut` to the mutex, and `into_inner` gives it up.
use core::sync::Mutex;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut m = Mutex::new(5i64);
    *m.get_mut() = 6;
    let doubled = m.lock(|x: &mut i64| {
        *x = *x * 2;
        *x
    });
    let mut out = os.stdout;
    out.line(&doubled)?;        // expect: 12
    out.line(&m.into_inner())?; // expect: 12
    Result::Ok(())
}
