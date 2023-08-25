# Cleanroom

Cleanroom is a way to build immutable Linux distribution images backed by
strong cryptography. Any change to the image will be detected.

This is the second iteration written in Rust: The first was written in Python
and can be found in the [cleanroom repository](https://github.com/cleanroom-team/cleanroom)

## Installation

Cleanroom consists of one binary called `cleanroom` and configuration for the
systems you want to create. Just grad the file from the github release page.

Alternatively you can clone this repository and run

```
cargo run --release -p cli
```
