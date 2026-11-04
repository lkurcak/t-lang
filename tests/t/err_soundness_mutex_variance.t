// A mutex is invariant in what it holds, like the cell inside it: through
// a `&Mutex<&'a str>`, `lock` could put in a reference that doesn't live
// as long as the mutex's type says.
use alloc::string::String;
use core::sync::Mutex;

fn put<'a>(m: &Mutex<&'a str>, s: &'a str) {
    m.lock(|r: &mut &'a str| *r = s);
}

fn main() with alloc {
    let m: Mutex<&'static str> = Mutex::new("static");
    {
        let s = String::from("short");
        put(&m, s.as_str());
        // error: `s` doesn't live long enough
    }
    let _ = m.into_inner();
}
