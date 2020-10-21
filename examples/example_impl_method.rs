use trace::trace;

trace::init_depth_var!();

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

#[cfg(test)]
#[macro_use]
mod trace_test;

#[cfg(test)]
trace_test!(test_impl_method, main());
