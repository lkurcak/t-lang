// Without `sealed`, another package could implement `Small` for `char`,
// so `impl<T: Small> Describe for T` and `impl Describe for char` overlap
// (tests/t/sealed_negative.t has it sealed).
trait Small {}

impl Small for u8 {}

impl Small for bool {}

trait Describe {
    fn describe(&self) -> i64;
}

impl<T: Small> Describe for T {
    fn describe(&self) -> i64 { 1 }
}

impl Describe for char { // error: overlap
    fn describe(&self) -> i64 { 2 }
}

impl<'a, T: Describe> Describe for &'a T {
    fn describe(&self) -> i64 { 20 + (**self).describe() }
}

impl<T: Describe> Describe for Option<T> {
    fn describe(&self) -> i64 {
        match self {
            Option::Some(x) => 10 + x.describe(),
            Option::None => 10,
        }
    }
}

fn main() {}
