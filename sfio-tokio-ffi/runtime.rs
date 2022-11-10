use std::future::Future;

use tokio::runtime::Handle;

use crate::ffi;

#[derive(Copy, Clone, Debug)]
pub enum RuntimeError {
    RuntimeDestroyed,
    CannotBlockWithinAsync,
    FailedToCreateRuntime,
}

pub struct Runtime {
    pub(crate) inner: std::sync::Arc<tokio::runtime::Runtime>,
}

impl Runtime {
    fn new(inner: tokio::runtime::Runtime) -> Self {
        Self {
            inner: std::sync::Arc::new(inner),
        }
    }

    pub fn handle(&self) -> RuntimeHandle {
        RuntimeHandle {
            inner: std::sync::Arc::downgrade(&self.inner),
        }
    }
}

#[derive(Clone)]
pub struct RuntimeHandle {
    inner: std::sync::Weak<tokio::runtime::Runtime>,
}

impl RuntimeHandle {
    pub fn block_on<F: Future>(&self, future: F) -> Result<F::Output, RuntimeError> {
        let inner = self
            .inner
            .upgrade()
            .ok_or(RuntimeError::RuntimeDestroyed)?;

        if Handle::try_current().is_ok() {
            return Err(RuntimeError::CannotBlockWithinAsync);
        }
        Ok(inner.block_on(future))
    }

    pub fn spawn<F>(&self, future: F) -> Result<(), RuntimeError>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static,
    {
        let inner = self
            .inner
            .upgrade()
            .ok_or(RuntimeError::RuntimeDestroyed)?;

        inner.spawn(future);
        Ok(())
    }
}

fn build_runtime<F>(f: F) -> std::result::Result<tokio::runtime::Runtime, std::io::Error>
    where
        F: Fn(&mut tokio::runtime::Builder) -> &mut tokio::runtime::Builder,
{
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    f(&mut builder).enable_all().build()
}

pub(crate) unsafe fn runtime_create(
    config: ffi::RuntimeConfig,
) -> Result<*mut crate::runtime::Runtime, RuntimeError> {
    let num_threads = if config.num_core_threads == 0 {
        num_cpus::get()
    } else {
        config.num_core_threads as usize
    };

    tracing::info!("creating runtime with {} threads", num_threads);
    let runtime = build_runtime(|r| r.worker_threads(num_threads))
        .map_err(|_| RuntimeError::FailedToCreateRuntime)?;
    Ok(Box::into_raw(Box::new(Runtime::new(runtime))))
}

pub(crate) unsafe fn runtime_destroy(runtime: *mut crate::runtime::Runtime) {
    if !runtime.is_null() {
        drop(Box::from_raw(runtime));
    };
}

