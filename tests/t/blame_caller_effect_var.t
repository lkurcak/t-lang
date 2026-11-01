// An effect variable can stand for an abort of a function that blames its
// caller, so the attribute is allowed with one: here `W::e`, and `Loud`'s
// `write_str` panics at the `fmt` call's caller.
use core::fmt::Write;

#[derive(Discard)]
struct Loud {}

impl Write for Loud {
    type Error = !;
    effect e = panic;

    #[blame_caller]
    fn write_str(&mut self, s: &str) -> Result<(), !> with panic {
        assert(s.len() == 0);
        Result::Ok(())
    }

    fn write_char(&mut self, _: char) -> Result<(), !> { Result::Ok(()) }
}

#[blame_caller]
fn say<W: Write>(w: &mut W) -> Result<(), W::Error> with W::e { w.write_str("hi") }

fn main() with panic {
    let mut w = Loud {};
    say(&mut w).into_ok();
}
// error: blame_caller_effect_var.t:27:5
