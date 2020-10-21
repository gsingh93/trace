extern crate trace;

#[macro_use]
mod trace_test;

use trace::trace;

trace::init_depth_var!();

trace_test!(test_pretty, {
    foo(Foo("Foo".to_string()));
});

#[derive(Debug)]
struct Foo(String);

#[trace(pretty)]
fn foo(a: Foo) -> Foo {
    a
}
