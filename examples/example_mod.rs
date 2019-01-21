#![feature(custom_attribute, plugin)]
#![plugin(trace)]
#![trace]

fn main() {
    foo();
    let foo = Foo;
    foo.bar();
}

fn foo() {
    println!("I'm in foo!");
}

struct Foo;
impl Foo {
    fn bar(&self) {}
}
