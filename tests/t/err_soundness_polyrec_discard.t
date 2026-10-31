// Through a `discard`, which nothing names: it's made with each copy of
// its type, as it's called wherever a value of it is dropped. `S<T>`'s
// makes an `S<Option<T>>`, and drops it, which needs `S<Option<T>>`'s,
// which makes an `S<Option<Option<T>>>`, and so on. Before step 29 this
// was accepted, and monomorphising it never ended (see
// docs/29-closed-and-sealed-traits.md).
struct S<T> { t: T }

impl<T: Discard> Discard for S<T> {
    fn discard(self) {
        let _x = S { t: Option::Some(self.t) };
    }
}

fn main() {
    let _s = S { t: 1i64 };
}
// error: polymorphic recursion: this use of `S` with `T` = `Option<T>` leads back to `<S<T> as Discard>::discard`
