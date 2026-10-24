// In a type's own `discard`, `self` isn't thrown away by calling it again:
// what's left of it is dropped field by field, each by its own type's
// `discard`.
use alloc::string::String;

struct Named<'a> { out: &'a mut std::io::Stdout, name: String }

impl<'a> Discard for Named<'a> {
    fn discard(self) {
        let _ = self.out.line(&self.name.as_str());
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let _n = Named { out: &mut out, name: String::from("bye") };
    Result::Ok(())
} // expect: bye
