# Crazyradio webusb rust driver

Driver to use Crazyradio in Rust using the WebUSB API.

This driver is intended to be used when targetting the web browser with Wasm.
It implements the async API of the native [Crazyradio crate](https://crates.io/crates/crazyradio).

## Versioning

This repos follows the version of the [crazyradio](https://crates.io/crates/crazyradio) crate.
For example, version *0.2.x* of this crate implements the same async API as version *0.2.x*
of the `crazyradio` crate.

This allows to 'duck type' this crate and the `crazyradio` crate, for example
this is done in the `crazyflie-link` crate:

``` rust
#[cfg(feature = "native")]
pub(crate) use crazyradio;
#[cfg(feature = "webusb")]
pub(crate) use crazyradio_webusb as crazyradio;
```

## Compiling requirement

Webusb is still an unstable API and so the `web-sys` crate requires a specific
unstable build flag to compile it in. This can be done by adding a  `.cargo/config.toml`
file to your project with the content:

``` toml
[build]
rustflags = ["--cfg=web_sys_unstable_apis"]
```

## Limitations

This lib can only open one radio when using the `Crazyradio::open_nth_async()` function.
This is only a UI limitation, not a hard one, see ticket #1 if you are interested
in the problem and in helping resolving it.

## Running tests

A couple of tests can be run in a web browser using `wasm_bindgen_test`:

```
wasm-pack test --chrome
```

To run, there should be at least one Crazyradio connected and paired on the test url `http://localhost:8000`.
The easiest to achieve that is to open the Crazyradio dongle from a development
[Crazyflie web client](https://github.com/ataffanel/crazyflie-client-web).