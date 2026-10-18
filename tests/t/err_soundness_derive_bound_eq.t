// A derived impl needs exactly what makes its fields implement the trait,
// which can be another trait: `Q<T>: Eq` needs `T: Show`, so `W<T>`'s
// derived `Eq` does too. It used to need `T: Eq` instead, so `W<int>`
// passed, and monomorphisation then couldn't find `int: Show`.
trait Show {
    fn show(&self) -> int;
}

#[derive(Discard)]
struct Q<T> { x: T }

impl<T: Show> Eq for Q<T> {
    fn eq(&self, o: &Q<T>) -> bool { self.x.show() == o.x.show() }
}

#[derive(Discard, Eq)]
struct W<T> { q: Q<T> }

fn main() {
    let a = W { q: Q { x: 1 } };
    let b = W { q: Q { x: 1 } };
    print_bool(a == b);
}
// error: cannot compare `W<int>` with `==`: that needs `int: Show`
