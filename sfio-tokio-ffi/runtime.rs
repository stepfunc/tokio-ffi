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
    inner: Option<tokio::runtime::Runtime>,
    shutdown_timeout: Option<Duration>,
}

impl Runtime {
    fn new(inner: tokio::runtime::Runtime) -> Self {
        Self {
            inner: Some(inner),
            shutdown_timeout: None,
        }
    }

    pub(crate) fn handle(&self) -> RuntimeHandle {
        RuntimeHandle {
            inner: self.inner.as_ref().unwrap().handle().clone(),
        }
    }

    pub(crate) fn enter(&self) -> tokio::runtime::EnterGuard<'_> {
        self.inner.as_ref().unwrap().enter()
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        let runtime = self.inner.take().unwrap();
        match self.shutdown_timeout {
            Some(timeout) => {
                tracing::info!("beginning runtime shutdown (timeout == {timeout:?})");
                runtime.shutdown_timeout(timeout);
                tracing::info!("runtime shutdown complete");
            }
            None => {
                tracing::info!("beginning runtime shutdown (no timeout)");
                drop(runtime);
                tracing::info!("runtime shutdown complete");
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct RuntimeHandle {
    inner: Handle,
}

impl RuntimeHandle {
    pub(crate) fn new(inner: Handle) -> Self {
        Self { inner }
    }

    pub(crate) fn block_on<F: Future>(&self, future: F) -> Result<F::Output, RuntimeError> {
        if Handle::try_current().is_ok() {
            return Err(RuntimeError::CannotBlockWithinAsync);
        }
        Ok(self.inner.block_on(future))
    }

    pub(crate) fn spawn<F>(&self, future: F) -> Result<(), RuntimeError>
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

