extern crate trace;

use trace::trace;

static mut DEPTH: usize = 0;

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
