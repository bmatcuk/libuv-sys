[![Build Status](https://travis-ci.com/bmatcuk/libuv-sys.svg?branch=master)](https://travis-ci.com/bmatcuk/libuv-sys)

# libuv-sys
libuv-sys provides a thin, low-level binding to the [libuv] library.

## Versioning and Usage
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

When declaring your dependencies, you'll probably want to specify a minimum
major/minor version of libuv-sys that'll either accept any patch version (using
the tilde operator), or any minor/patch version (using the caret operator, or
no operator at all):

```toml
[dependencies]
libuv-sys = "1.30"
```

If you need a specific patch version of libuv, check the [releases] page to
find the version of libuv-sys that corresponds to the patch of libuv that you
need.

[libuv]: https://libuv.org/
[releases]: https://github.com/bmatcuk/libuv-sys/releases
[semantic versioning]: https://semver.org/
