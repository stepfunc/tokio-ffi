use std::future::Future;
use std::time::Duration;

use tokio::runtime::Handle;

use crate::ffi;

#[derive(Copy, Clone, Debug)]
pub enum RuntimeError {
    RuntimeDestroyed,
    CannotBlockWithinAsync,
    FailedToCreateRuntime,
}

pub struct Runtime {
    pub(crate) inner: Option<tokio::runtime::Runtime>,
    pub(crate) shutdown_timeout: Option<Duration>,
}

impl Runtime {
    fn new(inner: tokio::runtime::Runtime) -> Self {
        Self {
            inner: Some(inner),
            shutdown_timeout: None,
        }
    }

    pub fn handle(&self) -> RuntimeHandle {
        RuntimeHandle {
            inner: self.inner.as_ref().unwrap().handle().clone(),
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if let (Some(runtime), Some(timeout)) = (self.inner.take(), self.shutdown_timeout) {
            runtime.shutdown_timeout(timeout)
        }
    }
}

#[derive(Clone)]
pub struct RuntimeHandle {
    inner: Handle,
}

impl RuntimeHandle {
    pub fn block_on<F: Future>(&self, future: F) -> Result<F::Output, RuntimeError> {
        if Handle::try_current().is_ok() {
            return Err(RuntimeError::CannotBlockWithinAsync);
        }
        Ok(self.inner.block_on(future))
    }

    pub fn spawn<F>(&self, future: F) -> Result<(), RuntimeError>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static,
    {
        self.inner.spawn(future);
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

pub(crate) unsafe fn runtime_set_shutdown_timeout(instance: *mut Runtime, timeout: Duration) {
    if let Some(rt) = instance.as_mut() {
        rt.shutdown_timeout = Some(timeout);
    }
}

