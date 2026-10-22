// A trait, a type and an impl of one for the other can each be in a
// different module. Calling `x.show()` needs `Show` in scope; `Show::show`
// by path doesn't. Generic code in one module is used at a type from
// another, and the impl chosen is the same either way.
mod show {
    pub trait Show {
        fn show(&self) -> i64 with panic;
    }

    pub fn twice<T: Show>(x: &T) -> i64 with panic { x.show() * 2 }
}

mod shapes {
    #[derive(Discard)]
    pub struct Sq { side: i64 }

    pub fn sq(side: i64) -> Sq { Sq { side } }

    pub fn side(s: &Sq) -> i64 { s.side }
}

mod impls {
    use crate::show::Show;
    use crate::shapes::{Sq, side};

    impl Show for Sq {
        fn show(&self) -> i64 with panic { side(self) * side(self) }
    }
}

use show::Show;

fn main() with panic {
    let s = shapes::sq(3);
    print(s.show());               // expect: 9
    print(show::twice(&s));        // expect: 18
    print(show::Show::show(&s));   // expect: 9
}
