// A program that needs nothing of std, so it builds for the freestanding
// target too (docs/39-freestanding.md): its heap is the platform's, and
// what it shows, it shows by how it ends. Lists, boxes, strings and arenas
// all get their memory through the platform's functions.
use alloc::arena::Arena;
use alloc::boxed::Box;
use alloc::list::List;
use alloc::string::String;

#[derive(Discard)]
struct Node<'a> { value: u32, next: Option<&'a Node<'a>> }

fn main() {
    let mut l = List::new();
    for i in 0..1000u32 { l.push(i) }
    let mut sum = 0u64;
    for x in l.iter() { sum = sum + (*x as u64) }
    let b = Box::new(sum);
    let nodes = Arena::new();
    let first: &Node = nodes.alloc(Node { value: 1, next: Option::None });
    let second: &Node = nodes.alloc(Node { value: 2, next: Option::Some(first) });
    let mut s = String::of(&*b);
    s.push(' ');
    let n = String::of(&(second.value + nodes.len() as u32));
    s.push_str(n.as_str());
    while l.len() > 10 { let _ = l.pop(); }
    core::panic::panic(s.as_str())
}
// error: runtime error: panicked: 499500 4
