//! A file descriptor's number is only the package's to make up: its field
//! is `pub`, seen in the package, not `export`.
export struct Fd { pub raw: i64 }
export fn open() -> Fd { Fd { raw: 3 } }
export fn close(f: Fd) -> i64 { let Fd { raw } = f; raw }
