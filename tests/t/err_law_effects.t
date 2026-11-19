// A law says nothing of what the method may do: its effects are the
// method's (docs/88-iterator-laws.md).
trait A {
    fn f(&self) -> usize with panic;
}

trait B: A {
    law fn f(&self) -> { v: usize | v < 3 };
}

fn main() {}
// error: its effects should be `{panic}`, as the method's are
