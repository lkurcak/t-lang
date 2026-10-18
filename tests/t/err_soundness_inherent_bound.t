// An inherent impl's bounds are needed where its methods are called.
struct Fd { n: int }

struct W<T> { x: T }

impl<T: Discard> W<T> {
    fn drop_it(self) {}
}

fn main() {
    W { x: Fd { n: 1 } }.drop_it();
}
// error: `W<T>::drop_it` needs its `T` to implement `Discard`, and `Fd` doesn't
