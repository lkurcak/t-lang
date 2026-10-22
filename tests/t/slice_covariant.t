// A `&[T]` is covariant in its lifetime and in `T`, as a `&T` is; a
// `&mut [T]` only in its lifetime.
fn shorter<'a, 'b>(s: &'b [&'static int]) -> &'b [&'a int] { s }

fn shorter_mut<'a, 'b: 'a, T>(s: &'b mut [T]) -> &'a mut [T] { s }

fn main() {
    let e = shorter(core::slice::empty());
    print(e.len()); // expect: 0
    let m: &mut [int] = shorter_mut(core::slice::empty_mut());
    print(m.len()); // expect: 0
}
