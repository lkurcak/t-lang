/// What can say what it is as a number.
export trait Number {
    fn number(&self) -> i64;
}

impl Number for crate::Counter {
    fn number(&self) -> i64 { self.get() }
}
