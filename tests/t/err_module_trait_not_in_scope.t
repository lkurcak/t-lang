// `x.show()` only looks at traits in scope here: `Show` isn't.
mod show {
    pub trait Show {
        fn show(&self) -> i64;
    }

    impl Show for super::P {
        fn show(&self) -> i64 { self.n }
    }
}

#[derive(Discard)]
struct P { n: i64 }

fn main() {
    let p = P { n: 1 };
    print(p.show());
}
// error: `P` has no method `show` in scope; trait `show::Show` has one: import it with `use show::Show;`
