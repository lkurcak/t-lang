// A trait impl's method can blame its caller: the method call, after
// static dispatch, in generic code as in any other.
trait Check {
    fn check(&self) with panic;
}

#[derive(Discard)]
struct Even { n: u8 }

impl Check for Even {
    #[blame_caller]
    fn check(&self) with panic {
        if self.n % 2 != 0 { panic("odd") }
    }
}

#[blame_caller]
fn all<T: Check>(xs: &[T]) with panic {
    let mut i: usize = 0;
    while i < xs.len() {
        xs[i].check();
        i = i + 1;
    }
}

fn main() {
    let xs = [Even { n: 2 }, Even { n: 3 }];
    xs[0].check();
    all(xs.as_slice());
}
// error: blame_caller_method.t:29:5
