pub struct Fd { raw: i64 }
pub fn close(f: Fd) -> i64 { let Fd { raw } = f; raw }
