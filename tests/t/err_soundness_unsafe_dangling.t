// What invariance prevents: a library's own cell, over `Unsafe`, given a
// reference to a string that's gone before the cell is read. Were it
// covariant, `&MyCell<&'long str>` would pass as `&MyCell<&'short str>`.
use alloc::string::String;
use core::cell::Unsafe;

#[derive(Discard where T: Discard)]
struct MyCell<T> { value: Unsafe<T> }

impl<T: Copy> MyCell<T> {
    fn put(&self, v: T) {
        // SAFETY: no loan of what's in it lives across this.
        unsafe {
            *self.value.as_mut() = v;
        }
    }

    fn get(&self) -> T {
        // SAFETY: no `as_mut` loan lives across this.
        unsafe { *self.value.as_ref() }
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let c: MyCell<&str> = MyCell { value: Unsafe::new("long") };
    {
        let s = String::from("short");
        c.put(s.as_str());
    }
    out.line(&c.get())?;
    Ok(())
}
// error: `s` doesn't live long enough
