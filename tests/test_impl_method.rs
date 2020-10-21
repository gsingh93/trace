extern crate trace;

#[macro_use]
mod trace_test;

use trace::trace;

trace::init_depth_var!();

trace_test!(test_impl_method, {
    let foo = Foo;
    Foo::foo(2);
    foo.bar(7);
});

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
