[![Build Status](https://travis-ci.com/bmatcuk/libuv-sys.svg?branch=master)](https://travis-ci.com/bmatcuk/libuv-sys)
[![Latest Release](https://img.shields.io/crates/v/libuv-sys2)](https://crates.io/crates/libuv-sys2)

# libuv-sys2
libuv-sys2 provides ffi bindings to the [libuv] library.

## Why libuv-sys2?
I am committed to maintaining libuv-sys2. In fact, releases are largely
automated. When [libuv] releases a new version, you may see a corresponding
release of libuv-sys2 within minutes. Sometimes the release cannot be
completely automated, however. In these cases, where I need to manually make
some changes, I aim to have a release within 24-hours.

## Versioning
libuv-sys2 uses [semantic versioning], like libuv. Major and minor versions of
libuv-sys2 are bound to specific major/minor versions of libuv, ie, libuv-sys2
v1.30.x corresponds to v1.30.x of libuv. The patch version of libuv-sys2 will
change anytime libuv updates, _or_ if libuv-sys2 updates.

## Getting Started
Include libuv-sys2 as a dependency in your Cargo.toml. It is recommended to use
the tilde operator when specifying your dependency on libuv-sys2 so that you'll
automatically received the latest bug fixes without any breaking API changes.
For example:

```toml
[dependencies]
libuv-sys2 = "~1.31.1"
```

This would be the same as specifying the version as `>= 1.31.1, < 1.35.0`.

If you need a specific patch version of libuv, check the [releases] page to
find the version of libuv-sys2 that corresponds to the patch of libuv that you
need.

Under the hood, libuv-sys2 uses [bindgen] to generate the bindings to [libuv].
If you're having trouble compiling libuv-sys2, check out the [bindgen]
documentation to make sure you have all the required software installed. For
example, on Windows, you'll need to use the msvc toolchain to compile
libuv-sys2.

## Usage
Import the library in your project:

```rust
#[macro_use]
extern crate libuv_sys2;
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
working, but you're pretty sure you're doing the right thing as far as [libuv]
is concerned, make sure your data has a stable memory address.

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

## Cross-Platform Considerations
It appears the type of uv_buf_t.len is different on Windows. A simple solution
is to use a usize (which appears to be the default elsewhere) and then any
place that you read from or write to a uv_buf_t.len, simply add a `as _` to the
end and the compiler will do the right thing. For example:

```rust
let buf: uv_buf_t = { base: my_ptr, len: my_len as _ };
let buflen: usize = buf.len as _;
```

Speaking of Windows, because [bindgen] is used to generate the bindings, you'll
need to use rust's msvc toolchain to compile libuv-sys2!

[bindgen]: https://rust-lang.github.io/rust-bindgen/
[examples]: https://github.com/bmatcuk/libuv-sys/tree/master/examples
[libuv's documentation]: http://docs.libuv.org
[libuv]: https://libuv.org/
[releases]: https://github.com/bmatcuk/libuv-sys/releases
[semantic versioning]: https://semver.org/
