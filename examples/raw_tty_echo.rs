#[macro_use]
extern crate libuv_sys2;

use libuv_sys2::{
    uv_buf_t, uv_close, uv_default_loop, uv_err_name, uv_file, uv_handle_get_data,
    uv_handle_set_data, uv_handle_t, uv_is_closing, uv_loop_close, uv_loop_t, uv_read_start,
    uv_read_stop, uv_run, uv_run_mode_UV_RUN_DEFAULT, uv_stream_t, uv_strerror, uv_tty_init,
    uv_tty_mode_t_UV_TTY_MODE_RAW, uv_tty_reset_mode, uv_tty_set_mode, uv_tty_t, uv_walk, uv_write,
    uv_write_t,
};
use std::error::Error;
use std::ffi::CStr;
use std::fmt;
use std::mem;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

const STDIN_FILENO: uv_file = 0;
const STDOUT_FILENO: uv_file = 1;

/// An error returned by libuv
#[derive(Clone, Debug)]
struct UVError {
    func: String,
    code: c_int,
}

impl fmt::Display for UVError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let err = CStr::from_ptr(uv_strerror(self.code)).to_string_lossy();
            let name = CStr::from_ptr(uv_err_name(self.code)).to_string_lossy();
            write!(f, "Error calling {}: {} ({})", self.func, err, name)
        }
    }
}

impl Error for UVError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// A result that may be a UVError
type Result<T> = std::result::Result<T, UVError>;

/// Turns the return code from a libuv function into a Result.
macro_rules! uvret {
    ($func:ident$args:tt) => {
        match $func$args {
            code if code < 0 => Err(UVError { func: stringify!($func).to_owned(), code }),
            _ => Ok(())
        }
    };
}

/// All of the structs that we need to pass back-and-forth with libuv must live in a predictable,
/// static place. Everything in rust lives, by default, on the stack. We want our libuv structs to
/// live on the heap so that they will have a static memory address. This Globals struct
/// encompasses all of our libuv structs for this purpose.
struct Globals {
    tty: uv_tty_t,
    ttyout: uv_tty_t,
    write_req: uv_write_t,
    write_buf: uv_buf_t,
    err: Option<UVError>,
}

impl Globals {
    /// Initialize a Globals struct on the heap and return a pointer to it.
    ///
    /// # Safety
    /// Because this method initializes on the stack and returns a raw pointer, the *caller* is
    /// responsible for cleaning it up later. To do that, one must use code such as:
    ///
    /// ```
    /// let r#loop = uv_default_loop();
    /// let globals = Globals::init(r#loop)?;
    /// // ...
    /// mem::drop(Box::from_raw(globals));
    /// ```
    ///
    /// Failure to do so will leak memory.
    unsafe fn init(r#loop: *mut uv_loop_t) -> Result<*mut Globals> {
        // Allocate memory for our structs. These variables are on the stack!
        let tty = mem::zeroed();
        let ttyout = mem::zeroed();
        let write_req = mem::zeroed();
        let write_buf = uv_buf_t {
            base: ptr::null_mut(),
            len: 0,
        };

        // Allocate the Globals struct and move it to the heap. NOTE: this makes a bit-wise copy of
        // the variables above onto the heap. The variables above are still on the stack!
        let globals = Box::into_raw(Box::new(Globals {
            tty,
            ttyout,
            write_req,
            write_buf,
            err: None,
        }));

        // From this point forward, we can use raw pointers to the structs inside our heap-
        // allocated Globals instance. We must _not_ use the variables above that are still on the
        // stack!

        // Initialize tty for stdin and ttyout for stdout
        uvret!(uv_tty_init(r#loop, &mut (*globals).tty, STDIN_FILENO, 0))?;
        uvret!(uv_tty_init(
            r#loop,
            &mut (*globals).ttyout,
            STDOUT_FILENO,
            0
        ))?;

        // uv_handle_t types have a data pointer we can use to store any arbitrary data. We'll
        // store a pointer to our Globals struct there so we can get at it from callbacks.
        (*globals).set_on_handle(uv_handle!(&mut (*globals).tty));
        (*globals).set_on_handle(uv_handle!(&mut (*globals).ttyout));
        (*globals).set_on_handle(uv_handle!(&mut (*globals).write_req));

        Ok(globals)
    }

    /// Retrieve the pointer to our Globals struct from a uv_handle_t.
    unsafe fn get_from_handle(handle: *const uv_handle_t) -> *mut Globals {
        uv_handle!(uv_handle_get_data(handle))
    }

    /// Store a pointer to our Globals struct on a uv_handle_t.
    unsafe fn set_on_handle(&mut self, handle: *mut uv_handle_t) {
        uv_handle_set_data(handle, self as *mut Globals as *mut c_void);
    }
}

/// Stop the libuv loop by stopping all of the handles that we've started.
unsafe fn stop(globals: *mut Globals) -> Result<()> {
    uvret!(uv_read_stop(uv_handle!(&mut (*globals).tty)))
}

/// This function is used by uv_read_start to allocate memory for the read.
unsafe extern "C" fn alloc_cb(
    _handle: *mut uv_handle_t,
    suggested_size: usize,
    buf: *mut uv_buf_t,
) {
    // Our allocation is pretty "dumb" here: we're just going to allocate a vec of the suggested
    // size. In a production app, we'd probably do something fancier to avoid allocating all the
    // time. Once the vec is allocated, we stick a raw pointer in the uv_buf_t and then call
    // mem::forget() on it so that it will not be deallocated when this method returns.
    let mut data = Vec::with_capacity(suggested_size as _);
    (*buf).base = data.as_mut_ptr() as *mut c_char;
    (*buf).len = suggested_size as _;
    mem::forget(data);
}

/// This function is called by uv_write when the write has finished. We'll use it to deallocate the
/// buffer we created. See write() below.
unsafe extern "C" fn write_cb(req: *mut uv_write_t, _status: c_int) {
    // reconstruct the vec from the buffer and drop it
    let globals = Globals::get_from_handle(uv_handle!(req));
    let len = (*globals).write_buf.len as _;
    mem::drop(Vec::from_raw_parts((*globals).write_buf.base, len, len));
}

/// Write to ttyout.
unsafe fn write(globals: *mut Globals, mut data: Vec<u8>, len: usize) -> Result<()> {
    // This function takes ownership of the vec that is passed to it. It gets a raw pointer to the
    // underlying data, and then calls mem::forget on the vec so that it is not deallocated by
    // rust. It'll be deallocated later by write_cb() above when the write finishes.
    (*globals).write_buf.base = data.as_mut_ptr() as *mut c_char;
    (*globals).write_buf.len = len as _;
    mem::forget(data);

    uvret!(uv_write(
        uv_handle!(&mut (*globals).write_req),
        uv_handle!(&mut (*globals).ttyout),
        uv_handle!(&(*globals).write_buf),
        1,
        Some(write_cb),
    ))
}

type NREAD = isize;

/// When a read happens on the tty "stream", this callback is called.
unsafe extern "C" fn read_cb(stream: *mut uv_stream_t, nread: NREAD, buf: *const uv_buf_t) {
    let globals = Globals::get_from_handle(uv_handle!(stream));
    let mut end = false;
    if nread > 0 {
        // reconstruct the vec from the raw pointer. When this block ends, rust will automatically
        // deallocate the vec when it falls out of scope. We'll also allocate a new vec to hold the
        // output we'd like to create. Then we'll loop through the input and copy it over to the
        // output, creating "escapes" for characters that cannot be printed.
        let data: Vec<u8> =
            Vec::from_raw_parts((*buf).base as *mut u8, nread as usize, (*buf).len as _);
        let mut outdata = Vec::with_capacity((nread as usize) * 2 + 1);
        for chr in data {
            match chr {
                // the enter key sends \r, but we output \n to move down a line
                b'\r' => outdata.push(b'\n'),

                // control characters, ie ctrl+letter
                0..=0x1a if chr != b'\t' => {
                    outdata.push(b'^');
                    outdata.push(0x40 | chr);

                    if chr == 0x3 {
                        // ctrl+c
                        end = true;
                    }
                }

                // some misc keystrokes such as esc
                0x1b..=0x1f => {
                    outdata.push(b'^');
                    outdata.push(0x50 | chr);
                }

                // forward delete
                0x7f => {
                    outdata.push(b'\\');
                    outdata.push(b'd');
                }

                // anything else is printable
                _ => outdata.push(chr),
            }
        }

        // null terminate our "string"!
        outdata.push(0);

        // write the output to ttyout - write() takes ownership of outdata
        let len = outdata.capacity();
        let res = write(globals, outdata, len);
        if (*globals).err.is_none() {
            (*globals).err = res.err();
        }
    } else if nread < 0 {
        // an error occurred
        end = true;
        if (*globals).err.is_none() {
            (*globals).err = Some(UVError {
                func: "read_cb".to_owned(),
                code: nread as c_int,
            });
        }
    }

    // if we should "end" (due to ctrl+c or an error), stop the loop
    if end {
        let res = stop(globals);
        if (*globals).err.is_none() {
            (*globals).err = res.err();
        }
    }
}

/// This callback is used by uv_walk to close all of our handles during loop cleanup.
unsafe extern "C" fn walk_and_close_cb(handle: *mut uv_handle_t, _arg: *mut c_void) {
    if !uv_is_closing(handle) != 0 {
        uv_close(handle, None);
    }
}

/// Our main program is here... allocate and initialize, run the loop, cleanup.
unsafe fn run() -> std::result::Result<(), Box<dyn Error>> {
    // allocate our libuv structs on the heap
    let r#loop = uv_default_loop();
    let globals = Globals::init(r#loop)?;

    // set to raw mode and start reading on the tty stream
    uvret!(uv_tty_set_mode(
        uv_handle!(&mut (*globals).tty),
        uv_tty_mode_t_UV_TTY_MODE_RAW,
    ))?;
    uvret!(uv_read_start(
        uv_handle!(&mut (*globals).tty),
        Some(alloc_cb),
        Some(read_cb as _),
    ))?;

    // output a little welcome message at program start
    let data = "This program echoes anything you type! Try it out (Ctrl+C to quit): "
        .as_bytes()
        .to_vec();
    let len = data.capacity();
    write(globals, data, len)?;

    // start the loop - this blocks until the loop is stopped
    uvret!(uv_run(r#loop, uv_run_mode_UV_RUN_DEFAULT))?;

    // reset the tty mode
    uvret!(uv_tty_reset_mode())?;

    // mark all handles for closing then restart the loop... we need to start a new loop here so
    // that the handles will actually be closed. The loop should be fairly short-lived because all
    // it needs to do is close all the handles.
    uv_walk(r#loop, Some(walk_and_close_cb), ptr::null_mut());
    uvret!(uv_run(r#loop, uv_run_mode_UV_RUN_DEFAULT))?;

    // close the loop
    uvret!(uv_loop_close(r#loop))?;

    // deallocate our libuv structs on the heap
    let err = (*globals).err.clone();
    mem::drop(Box::from_raw(globals));

    if let Some(err) = err {
        Err(Box::new(err))
    } else {
        Ok(())
    }
}

fn main() {
    // run the program and print any errors
    if let Err(err) = unsafe { run() } {
        println!("{}", err);
    }
}
