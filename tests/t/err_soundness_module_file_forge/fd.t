pub struct Fd { raw: int }
pub fn close(f: Fd) -> int { let Fd { raw } = f; raw }
