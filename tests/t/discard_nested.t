// Dropping a list of strings frees each string, then the list's buffer;
// so does dropping a box, an option, or a struct that holds them, and a
// closure that captured one.
use alloc::boxed::Box;
use alloc::list::List;
use alloc::string::String;

#[derive(Discard)]
struct Names { all: List<String>, best: Option<Box<String>> }

fn keep<T: Discard>(_x: T) {}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut all = List::new();
    all.push(String::from("ada"));
    all.push(String::from("grace"));
    let names = Names { all, best: Option::Some(Box::new(String::from("barbara"))) };
    out.line(&names.all.len())?; // expect: 2
    keep(names);
    let s = String::from("closure");
    let f = move || s.len();
    out.line(&f()) // expect: 7
}
