# Cleanroom

Cleanroom is a way to build immutable Linux distribution images backed by
strong cryptography. Any change to the image will be detected.

This is the second iteration written in Rust: The first was written in Python
and can be found in the [cleanroom repository](https://github.com/cleanroom-team/cleanroom)

_Cleanroom is *linux only* and will not work on any other OS at this time_

## Requirements

You will need to have the following binaries on the system you run cleanroom on:

- The `cleanroom` command defined in this repository
- `/usr/bin/systemd-nspawn` to set up containers with
- `busybox` as a statically compiled binary as a OS agnostic run environment
  inside and outside of containers.
- `direnv` helps but is not strictly necessary. Without it you will need to
  source a `.env` file to configure your shell for `cleanroom`.

Everything else is done in containers managed or built by `cleanroom`.

## Installation

Cleanroom consists of one binary called `cleanroom` and configuration for the
systems you want to create. Binary builds are available on
[Github](https://github.com/cleanroom-team/cleanroom-rs/releases).

Alternatively you can clone this repository and run

```
cargo run --release -p cli -- <arguments>
```

## Getting started

Run `cleanroom initialize` in an empty directory to get an arch linux based
playground project to experiment with.

`cleanroom initialize` takes the following arguments for a more tailored
start:

- `--distribution` to use as a base. This defaults to `arch`
- `--busybox-binary` pointing to the busybox binary
- the directory to set up camp in (defaults to `.`) as its only positional
  argument

This will set up a playground for you to experiment in:-)

Once `cleanroom initialize` is done check the generated `.env` file and source
it into your running shell (`source .env` in bash).
