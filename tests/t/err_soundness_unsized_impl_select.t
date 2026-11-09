// An impl's parameter that isn't `unsized` can't stand for one that is:
// `impl Tr for &T` makes an `Option<T>`, which at `D = str` would be an
// `Option<str>`. So `&D` doesn't implement `Tr` here.
use core::fmt::Display;

trait Tr { fn tr(&self) -> u8; }

impl<'a, T: Display> Tr for &'a T {
    fn tr(&self) -> u8 {
        let _o: Option<T> = None;
        0
    }
}

fn use_tr<T: Tr + Discard>(t: T) -> u8 { t.tr() }

fn f<unsized D: Display>(x: &D) -> u8 { use_tr(x) }

fn main() {}
// error: `use_tr` needs its `T` to implement `Tr`, and `&D` doesn't
