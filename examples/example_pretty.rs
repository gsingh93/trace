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

#[cfg(test)]
#[macro_use]
mod trace_test;

#[cfg(test)]
trace_test!(test_pretty, main());
