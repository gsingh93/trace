use trace::trace;

fn main() {
    foo::foo();
    let foo = foo::Foo;
    foo.bar();
}

#[trace]
mod foo {
    pub(super) fn foo() {
        println!("I'm in foo!");
    }

    pub(super) struct Foo;
    impl Foo {
        pub(super) fn bar(&self) {}
    }
}
