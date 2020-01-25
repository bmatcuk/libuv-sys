#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// This macro simplifies casting a reference or raw pointer to a uv_SOMETHING_t as a raw pointer
/// to a uv_SOMETHING_ELSE_t. This is frequently necessary to cast a uv_SOMETHING_t to a
/// uv_handle_t, but may also be used in other situations (casting a &mut uv_tty_t to a *mut
/// uv_stream_t, for example). Really, this macro can be used to cast any reference or raw pointer
/// to a raw pointer of a different type.
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate libuv_sys2;
/// #
/// # use libuv_sys2::{uv_handle_t, uv_tty_t};
/// # use std::mem;
/// #
/// # fn main() {
/// #
/// let mut tty: uv_tty_t = unsafe { mem::zeroed() };
///
/// // without the macro, you'd need to cast the reference to a raw pointer of the
/// // same type, and then cast that as a raw pointer of the target type:
/// let handle: *mut uv_handle_t = &mut tty as *mut uv_tty_t as *mut uv_handle_t;
///
/// // the macro is much more wieldy:
/// let handle: *mut uv_handle_t = uv_handle!(&mut tty);
/// #
/// # }
/// ```
#[macro_export]
macro_rules! uv_handle {
    (&mut $a:expr) => {
        &mut $a as *mut _ as *mut _
    };
    (&$a:expr) => {
        &$a as *const _ as *const _
    };
    ($a:expr) => {
        $a as _
    };
}
