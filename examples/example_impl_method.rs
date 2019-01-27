extern crate trace;

use std::cell::Cell;
use trace::trace;

thread_local! {
    static DEPTH: Cell<usize> = Cell::new(0);
}

fn main() {
    let foo = Foo;
    Foo::foo(2);
    foo.bar(7);
}

struct Foo;
impl Foo {
    fn foo(b: i32) -> i32 {
        b
    }

    #[trace]
    fn bar(&self, a: i32) -> i32 {
        a
    }
}
