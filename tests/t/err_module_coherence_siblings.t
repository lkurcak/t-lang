// Impls are global wherever they are: two in different modules overlap.
trait Show {
    fn show(&self) -> int;
}

mod a {
    impl super::Show for int {
        fn show(&self) -> int { 1 }
    }
}

mod b {
    impl super::Show for int {
        fn show(&self) -> int { 2 }
    }
}

fn main() {}
// error: `int` implements `Show` twice
