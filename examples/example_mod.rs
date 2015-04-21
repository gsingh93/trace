#![feature(custom_attribute, plugin)]
#![plugin(trace)]
#![trace]

fn main() {
    foo();
}

fn foo() {
    println!("I'm in foo!");
}
