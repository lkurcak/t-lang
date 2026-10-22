// Impls are global wherever they are: two in different modules overlap.
trait Show {
    fn show(&self) -> i64;
}

mod a {
    impl super::Show for i64 {
        fn show(&self) -> i64 { 1 }
    }
}

mod b {
    impl super::Show for i64 {
        fn show(&self) -> i64 { 2 }
    }
}

fn main() {}
// error: `i64` implements `Show` twice
