// A derived impl's bounds can be of other traits than its own: whatever
// makes each field implement it.

trait Show {
    fn show(&self) -> i64;
}

impl Show for i64 {
    fn show(&self) -> i64 { *self % 10 }
}

#[derive(Discard)]
struct Q<T> { x: T }

impl<T: Show> Eq for Q<T> {
    fn eq(&self, o: &Q<T>) -> bool { self.x.show() == o.x.show() }
}

// Needs `T: Show`, and `T: Discard`.
#[derive(Discard, Eq)]
struct W<T> { q: Q<T> }

fn same<T: Show + Discard>(a: W<T>, b: W<T>) -> bool { a == b }

fn main() {
    print_bool(W { q: Q { x: 1i64 } } == W { q: Q { x: 11 } }); // expect: true
    print_bool(same(W { q: Q { x: 2i64 } }, W { q: Q { x: 3 } })); // expect: false
}
