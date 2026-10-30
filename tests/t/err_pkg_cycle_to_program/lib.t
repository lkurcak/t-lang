// A library can't depend on a package that depends on it.
// error: packages can't depend on each other: the package in `.` depends on `inner`, which depends on the package in `.`
export fn g() -> i64 { inner::f() }
