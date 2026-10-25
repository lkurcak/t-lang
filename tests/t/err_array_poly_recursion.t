// An array's type grows with each call: no finite set of copies.
fn nest<T: Copy, const N: usize>(x: T, n: u64) {
    if n > 0 {
        nest::<[T; N], N>([x; N], n - 1);
    }
}

fn main() {}
// error: polymorphic recursion: `nest` uses itself with `T` = `[T; N]`
