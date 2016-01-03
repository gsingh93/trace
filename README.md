trace [![](https://meritbadge.herokuapp.com/trace)](https://crates.io/crates/trace) [![Build Status](https://travis-ci.org/gsingh93/trace.svg?branch=master)](https://travis-ci.org/gsingh93/trace)
-----

A syntax extension for tracing the execution of functions. Adding `#[trace]` to the top of any function will insert `println!` statements at the beginning and end of that function, notifying you of when that function was entered and exited and printing the argument and return values. This is useful for quickly debugging whether functions that are supposed to be called are actually called without manually inserting print statements.

Note that this extension requires all arguments to the function and the return value to have types that implement `Debug`. You can disable the printing of certain arguments if necessary (described below).

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

- Note the `depth` variable must be a global `static mut` variable, it's used for indenting the output. The only time it can be omitted is when `#[trace]` is applied to `mod`s, as described below.

- You can use `#[trace]` on `mod`s as well. To apply `#[trace]` to all functions in the current `mod`, put `#![trace]` (note the `!`) at the top of the file. When using `#[trace]` on `mod`s, the `depth` variable doesn't need to be defined (it's defined for you automatically). Note that the `depth` variable isn't shared between `mod`s, so indentation won't be perfect when tracing functions in multiple `mod`s.

- You can also use `#[trace]` on entire `impl`s or individual `impl` methods. See the `examples` folder for more details.

- If you use `#[trace]` on a `mod` or `impl` as well as on a method or function inside one of those structures, then only the outermost `#[trace]` is used.

## Optional Arguments

Trace takes a few optional arguments, described below:

- `prefix_enter` - The prefix of the `println!` statement when a function is entered. Defaults to `[+]`.

- `prefix_exit` - The prefix of the `println!` statement when a function is exited. Defaults to `[-]`.

- `enable` - When applied to a `mod` or `impl`, `enable` takes a list of function names to print, not printing any functions that are not part of this list. All functions are enabled by default. When applied to an `impl` method or a function, `enable` takes a list of arguments to print, not printing any arguments that are not part of the list. All arguments are enabled by default.

- `disable` - When applied to a `mod` or `impl`, `disable` takes a list of function names to not print, printing all other functions in the `mod` or `impl`. No functions are disabled by default. When applied to an `impl` method or a function, `disable` takes a list of arguments to not print, printing all other arguments. No arguments are disabled by default.

- `pause` - When given as an argument to `#[trace]`, execution is paused after each line of tracing output until enter is pressed. This allows you to trace through a program step by step.

Note that `enable` and `disable` can not be used together, and doing so will result in an error.

All of these options are covered in the `examples` folder.
