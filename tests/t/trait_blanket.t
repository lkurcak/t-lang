// A blanket impl, for every type that implements another trait. Proving
// `i64: Describe` needs `i64: Show`, for the same type: fine, as long as
// nothing leads back from `Show` to `Describe`.

trait Show {
    fn show(&self);
}

trait Describe {
    fn describe(&self);
}

impl<T: Show> Describe for T {
    fn describe(&self) {
        print(0i64);
        self.show();
    }
}

impl Show for i64 {
    fn show(&self) { print(*self) }
}

fn main() {
    5i64.describe();   // expect: 0
                    // expect: 5
}
