extern crate trace;

use trace::trace;

trace::init_depth_var!();

fn main() {
    foo(Foo("Foo".to_string()));
}

#[derive(Debug)]
struct Foo(String);

#[trace(pretty)]
fn foo(a: Foo) -> Foo {
    a
}
