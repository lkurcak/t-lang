// What `{x:>8}` is made of: `core::fmt::pad`, which can be called itself,
// with any `Display`, and a width and fill worked out as the program runs.
use core::fmt::{Align, display, pad};

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 42u8;
    let fill = '.';
    out.put(&pad(display(&n), 5, fill, Align::Right))?;
    out.line(&pad("|", 3, '-', Align::Center)) // expect: ...42-|-
}
