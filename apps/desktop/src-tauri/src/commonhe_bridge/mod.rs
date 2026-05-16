pub mod agent;
pub mod commands;
pub mod json;
pub mod payload;
pub mod provider;
pub mod shell;
pub mod status;

#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

#[cfg(test)]
pub(crate) fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
