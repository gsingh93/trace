trace [![](https://meritbadge.herokuapp.com/trace)](https://crates.io/crates/trace)
-----

A syntax extension for tracing the execution of functions. Adding `#[trace]` to the top of any function will insert `println!` statements at the beginning and end of that function, notifying you of when that function was entered and exited and printing the argument and return values. This is useful for quickly debugging whether functions that are supposed to be called are actually called without manually inserting print statements.

See the limitations section below for what this extension currently can't do.

Note that this extension requires all arguments to the function and the return value to have types that implement `Debug`.

## Installation

Add `trace = "*"` to your `Cargo.toml`.

## Example

Here is an example you can find in the examples folder. If you've cloned the project, you can run this with `cargo run --example example`.

```
#![feature(custom_attribute, plugin)]
#![plugin(trace)]

static mut depth: u32 = 0;

fn main() {
    foo(1, 2);
}

#[trace]
fn foo(a: i32, b: i32) {
    println!("I'm in foo!");
    bar((a, b));
}

#[trace(prefix_enter="[ENTER]", prefix_exit="[EXIT]")]
fn bar((a, b): (i32, i32)) -> i32 {
    println!("I'm in bar!");
    if a == 1 {
        2
    } else {
        b
    }
}
```

Output:
```
[+] Entering foo(a: 1, b: 2)
I'm in foo!
 [ENTER] Entering bar(a: 1, b: 2)
I'm in bar!
 [EXIT] Exiting bar = 2
[-] Exiting foo = ()
```

Note that you can customize the prefix of the `println!` statement with `prefix_enter` and `prefix_exit`. The `depth` variable must be a global `static mut` variable, it's used for indenting the output.

You can use `#[trace]` on `mod` items as well. To apply `#[trace]` to all functions in the current `mod`, put `#![trace]` (note the `!`) at the top of the file. When using `#[trace]` on `mod`s, the `depth` variable doesn't need to be defined (it's defined for you automatically). Note that the `depth` variable isn't shared between `mod`s, so indentation won't be perfect when tracing functions in multiple `mod`s.

## Limitations

- Currently, `#[trace]` is not supported on `impl` methods or `impl` declarations. This limitation will be lifted very soon.
- It would be nice to enable/disable the tracing of particular methods directly from the attribute on the `impl` or `mod` instead of the removing/adding the attribute to individual functions. This should be added soon.
- It would be nice to enable/disable the printing of arguments/return values from the attribute, especially since printing requires the types to implement `Debug`. This should be added soon.
- Trace only works for [certain types of patterns](https://github.com/gsingh93/trace/blob/master/src/lib.rs#L146) in the function arguments. If these patterns are too complicated, trace skips the entire function. This limitation is very tricky to fix, and there is no ETA on when it will be fixed.
