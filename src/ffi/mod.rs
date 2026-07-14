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

use std::ffi::CString;
use std::path::Path;

/// Owns a `whisper_context`, freeing it on drop.
#[derive(Debug)]
pub struct Context(*mut whisper_context);

// SAFETY: a Context is used single-threaded (one per Transcriber; whisper state is not Sync).
unsafe impl Send for Context {}

impl Context {
    pub fn from_file(path: &Path) -> crate::error::Result<Context> {
        let c =
            CString::new(path.to_string_lossy().as_bytes()).map_err(|e| crate::error::WhisperError::Config(e.to_string()))?;
        // SAFETY: default params; c is a valid NUL-terminated path for the call's duration.
        let ctx = unsafe {
            let params = whisper_context_default_params();
            whisper_init_from_file_with_params(c.as_ptr(), params)
        };
        if ctx.is_null() {
            return Err(crate::error::WhisperError::ModelNotFound {
                kind: crate::error::ModelKind::Whisper,
                path: path.to_path_buf(),
            });
        }
        Ok(Context(ctx))
    }

    /// Run full transcription. Returns Ok(()) on success, Err(Ffi(code)) on non-zero return.
    pub fn full(&mut self, lang: &str, threads: i32, token_timestamps: bool, pcm: &[f32]) -> crate::error::Result<()> {
        if pcm.len() > i32::MAX as usize {
            return Err(crate::error::WhisperError::Config(format!("audio too long: {} samples exceeds i32::MAX", pcm.len())));
        }
        let clang = CString::new(lang).map_err(|e| crate::error::WhisperError::Config(e.to_string()))?;
        // SAFETY: pcm and clang outlive the whisper_full call.
        let rc = unsafe {
            let mut params = whisper_full_default_params(whisper_sampling_strategy_WHISPER_SAMPLING_GREEDY);
            params.language = clang.as_ptr();
            params.n_threads = threads;
            params.print_progress = false;
            params.print_realtime = false;
            params.token_timestamps = token_timestamps;
            whisper_full(self.0, params, pcm.as_ptr(), pcm.len() as i32)
        };
        if rc != 0 {
            return Err(crate::error::WhisperError::Ffi(rc));
        }
        Ok(())
    }

    pub fn n_segments(&self) -> i32 {
        // SAFETY: self.0 is a valid context for the lifetime of self.
        unsafe { whisper_full_n_segments(self.0) }
    }

    pub fn segment_text(&self, i: i32) -> String {
        // SAFETY: i in [0, n_segments); whisper returns a valid or null NUL-terminated string.
        unsafe {
            let ptr = whisper_full_get_segment_text(self.0, i);
            if ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }

    /// Segment start/end in centiseconds (whisper t0/t1 units).
    pub fn segment_t0(&self, i: i32) -> i64 {
        // SAFETY: self.0 is a valid context; i in [0, n_segments).
        unsafe { whisper_full_get_segment_t0(self.0, i) }
    }
    pub fn segment_t1(&self, i: i32) -> i64 {
        // SAFETY: self.0 is a valid context; i in [0, n_segments).
        unsafe { whisper_full_get_segment_t1(self.0, i) }
    }

    /// Raw per-token timing for one segment: (text, t0_centiseconds, t1_centiseconds, probability).
    pub fn segment_tokens(&self, seg: i32) -> Vec<(String, i64, i64, f32)> {
        // SAFETY: seg in [0, n_segments); token indices in [0, n_tokens); pointers are whisper-owned.
        unsafe {
            let n = whisper_full_n_tokens(self.0, seg);
            let mut v = Vec::with_capacity(n.max(0) as usize);
            for j in 0..n {
                let td = whisper_full_get_token_data(self.0, seg, j);
                let tptr = whisper_full_get_token_text(self.0, seg, j);
                if tptr.is_null() {
                    continue;
                }
                let text = CStr::from_ptr(tptr).to_string_lossy().into_owned();
                v.push((text, td.t0, td.t1, td.p));
            }
            v
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: self.0 came from whisper_init_* and is freed exactly once.
        unsafe { whisper_free(self.0) }
    }
}
