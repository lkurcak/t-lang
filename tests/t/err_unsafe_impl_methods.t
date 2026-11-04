// `Send` and `Sync` have no methods: an `unsafe impl` of one is empty.
struct P { x: u32 }

unsafe impl Send for P {
    fn f() {}
// error: `f` isn't a method of trait `Send`
}

fn main() {}
