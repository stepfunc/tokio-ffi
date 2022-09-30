#[allow(dead_code)]
pub mod ffi;
mod runtime;

pub(crate) use runtime::*;

lazy_static::lazy_static! {
    static ref VERSION: std::ffi::CString = std::ffi::CString::new("0.1.0").unwrap();
}

fn version() -> &'static std::ffi::CStr {
    &VERSION
}

impl From<crate::RuntimeError> for std::os::raw::c_int {
    fn from(err: crate::RuntimeError) -> Self {
        match err {
            crate::RuntimeError::FailedToCreateRuntime => {
                crate::ffi::RuntimeError::RuntimeCreateFailed.into()
            }
            crate::RuntimeError::CannotBlockWithinAsync => {
                crate::ffi::RuntimeError::CannotBlockWithinAsync.into()
            }
            crate::RuntimeError::RuntimeDestroyed => {
                crate::ffi::RuntimeError::RuntimeDestroyed.into()
            }
        }
    }
}
