//! # Ultra-Lean ANSI Term Helper (Zero-Dependency)
//!
//! Minimalist macros for terminal coloring without the overhead of the `colored` crate.
//! Macros are zero-cost abstractions as they are expanded at compile-time.

#[allow(unused_macros)]
macro_rules! green {
    ($arg:expr) => {
        format!("\x1b[32m{}\x1b[0m", $arg)
    };
}

#[allow(unused_macros)]
macro_rules! yellow {
    ($arg:expr) => {
        format!("\x1b[33m{}\x1b[0m", $arg)
    };
}

#[allow(unused_macros)]
macro_rules! red {
    ($arg:expr) => {
        format!("\x1b[31m{}\x1b[0m", $arg)
    };
}

#[allow(unused_macros)]
macro_rules! blue {
    ($arg:expr) => {
        format!("\x1b[34m{}\x1b[0m", $arg)
    };
}

#[allow(unused_macros)]
macro_rules! cyan {
    ($arg:expr) => {
        format!("\x1b[36m{}\x1b[0m", $arg)
    };
}

#[allow(unused_macros)]
macro_rules! magenta {
    ($arg:expr) => {
        format!("\x1b[35m{}\x1b[0m", $arg)
    };
}

#[allow(unused_macros)]
macro_rules! bold {
    ($arg:expr) => {
        format!("\x1b[1m{}\x1b[0m", $arg)
    };
}

#[allow(unused_macros)]
macro_rules! dim {
    ($arg:expr) => {
        format!("\x1b[2m{}\x1b[0m", $arg)
    };
}

#[allow(unused_imports)]
pub(crate) use {blue, bold, cyan, dim, green, magenta, red, yellow};
