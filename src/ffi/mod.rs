//! Raw whisper.cpp FFI bindings. The ONLY module in this crate allowed to use `unsafe`.
#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::CStr;

/// Safe wrapper over `whisper_print_system_info` — proves the library links & calls without a model.
pub fn system_info() -> String {
    // SAFETY: whisper_print_system_info returns a pointer to a static, NUL-terminated C string.
    unsafe {
        let ptr = whisper_print_system_info();
        if ptr.is_null() {
            return String::new();
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}
