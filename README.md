[![Build Status](https://travis-ci.com/bmatcuk/libuv-sys.svg?branch=master)](https://travis-ci.com/bmatcuk/libuv-sys)
![Latest Release](https://img.shields.io/github/v/release/bmatcuk/libuv-sys?sort=semver)

# libuv-sys
libuv-sys provides a thin, low-level binding to the [libuv] library.

## Versioning
libuv-sys uses [semantic versioning], like libuv. Major and minor versions of
libuv-sys are bound to specific major/minor versions of libuv, ie, libuv-sys
v1.30.x corresponds to v1.30.x of libuv. The patch version is not necessarily
directly correlated, however. The patch version of libuv-sys will increase if a
new patch version of libuv is released, but it may also change if there is a
bug in the bindings, for example. There are also cases where higher patch
versions of libuv existed before any major/minor binding was created, so the
patch version of libuv-sys is _lower_ than the patch version of libuv that it
corresponds to. For example: v1.30.0 of libuv-sys corresponds to v1.30.1 of
libuv.

## Getting Started
libuv-sys is currently not available on crates.io because I consider it a "work
in progress". To be clear, everything appears to be working, but until I've
finished writing a program to fully exercise the bindings, I'm not comfortable
releasing it on crates.io. So, you'll need to depend on the github repo
directly. You can either refer to a specific tag or a branch. A tag will allow
you to specific a very specific version of libuv-sys, down to the patch number.
A branch will allow you to specify a major/minor version: you'll get the latest
patch version this way. This is the recommended way to specify the version.

```toml
[dependencies]
libuv-sys = { git = "https://github.com/bmatcuk/libuv-sys", branch = "v1.34.x" }
libuv-sys = { git = "https://github.com/bmatcuk/libuv-sys", tag = "v1.34.0" }
```

If you need a specific patch version of libuv, check the [releases] page to
find the version of libuv-sys that corresponds to the patch of libuv that you
need.

## Usage
Import the library in your project:

```rust
#[macro_use]
extern crate libuv_sys;
```

As this library is a thin binding to [libuv], the first thing you should do is
familiarize yourself with [libuv's documentation]. Once you're familiar with
the concepts, take a look at the [examples].

Some general advice: any data (such as libuv handle types) that you are
planning on passing into libuv should _probably_ be allocated on the heap
(using `Box`, for example). That way, they'll have a stable memory address.
Keep in mind that rust's default is to allocate things on the stack, which
means that their memory address changes if you pass it into a function or
return it from a function, and it will get deallocated once it falls out of
scope. It's very easy to write a progarm that will compile, but fail to run or
cause all sorts of undefined behavior because you'll be passing around a lot of
raw, unsafe pointers while interacting with the library. If something isn't
working, but you're pretty sure you're doing the right thing, make sure your
data has a stable memory address.

In addition to bindings for all of the [libuv] functionality, this library
provides one convenience macro: `uv_handle!`. This macro can be used to convert
any reference or raw pointer of one type, to a raw pointer of a different type.
This is frequently useful when using [libuv] to cast a `uv_SOMETHING_t` to a
`uv_handle_t`. For example:

```rust
let mut tty: uv_tty_t = unsafe { mem::zeroed() };

// without the macro, you'd need to cast the reference to a raw pointer of the
// same type, and then cast that as a raw pointer of the target type:
let handle: *mut uv_handle_t = &mut tty as *mut uv_tty_t as *mut uv_handle_t;

// the macro is much more wieldy:
let handle: *mut uv_handle_t = uv_handle!(&mut tty);
```

NOTE: this macro is only available in v1.34.1 and newer!

[examples]: https://github.com/bmatcuk/libuv-sys/tree/master/examples
[libuv's documentation]: http://docs.libuv.org
[libuv]: https://libuv.org/
[releases]: https://github.com/bmatcuk/libuv-sys/releases
[semantic versioning]: https://semver.org/
