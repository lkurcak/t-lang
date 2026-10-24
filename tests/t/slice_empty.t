// An empty slice points at nothing, so one can be made from nowhere, of
// any lifetime, even to change.
use core::slice;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s: &[i64] = slice::empty();
    out.line(&s.len())?;                        // expect: 0
    out.line(&s.first().is_none())?;       // expect: true
    out.line(&s.iter().next().is_none())?; // expect: true
    let m: &mut [bool] = slice::empty_mut();
    out.line(&m.get_mut(0).is_none())?; // expect: true
    out.line(&m.swap(0, 0))?;           // expect: false
    Result::Ok(())
}
