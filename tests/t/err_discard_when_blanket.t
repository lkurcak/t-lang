// `Discard` is implemented for a struct or enum, refined or not: for any
// other type it's built in already, or can't be.
impl<T> Discard for { t: T | true } {
    fn discard(self) {
        let _k = self;
    }
}

fn main() {}
// error: `Discard` can only be implemented for a struct or enum: it's built into `T` already, or can't be
