#[macro_use]
mod trace_test;

use trace::trace;

trace::init_depth_var!();

trace_test!(test_impl, {
    let foo = Foo;
    Foo::foo(2);
    foo.bar(7);
});

struct Foo;

#[trace]
impl Foo {
    fn foo(b: i32) -> i32 {
        b
    }

    fn bar(&self, a: i32) -> i32 {
        a
    }
}
