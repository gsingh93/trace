// This example is disabled due to the following error:
// error: cannot determine resolution for the attribute macro `trace`
//   --> examples/example_mod_inner_attribute_nightly.rs:10:4
//    |
// 11 | #![trace]
//    |    ^^^^^
//    |
//    = note: import resolution is stuck, try simplifying macro imports

// #![feature(custom_inner_attributes)]
// #![trace]

// use trace::trace;

// fn main() {
//     foo::foo();
//     let foo = foo::Foo;
//     foo.bar();
// }

// mod foo {
//     pub(super) fn foo() {
//         println!("I'm in foo!");
//     }

//     pub(super) struct Foo;
//     impl Foo {
//         pub(super) fn bar(&self) {}
//     }
// }

fn main() {}
