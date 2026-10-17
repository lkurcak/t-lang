// A blanket impl, for every type that implements another trait. Proving
// `int: Describe` needs `int: Show`, for the same type: fine, as long as
// nothing leads back from `Show` to `Describe`.

trait Show {
    fn show(&self);
}

trait Describe {
    fn describe(&self);
}

impl<T: Show> Describe for T {
    fn describe(&self) {
        print(0);
        self.show();
    }
}

impl Show for int {
    fn show(&self) { print(*self) }
}

fn main() {
    5.describe();   // expect: 0
                    // expect: 5
}
