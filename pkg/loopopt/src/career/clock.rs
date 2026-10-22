//! Host clocks, progress and phase counters.

/// Elapsed wall-clock ms: the host's `env.clock_ms` in wasm (performance.now()), else since the first native call.
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code, reason = "calls the host's clock import")]
pub(crate) fn now_ms() -> f64 {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        fn clock_ms() -> f64;
    }
    // SAFETY: the host provides `env.clock_ms` with this signature (performance.now()); it takes nothing and reads
    // no memory of ours.
    unsafe { clock_ms() }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn now_ms() -> f64 {
    static FIRST_CALL: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    FIRST_CALL.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
}

/// the plan's progress for the page's status line, at its phase boundaries (`plan::index_rule`): phase, k of n. In
/// wasm the host's `env.progress` (the worker posts it to the page while the plan runs); elsewhere nothing
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code, reason = "calls the host's progress import")]
pub(crate) fn progress(phase: u32, done: usize, total: usize) {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        #[link_name = "progress"]
        fn host_progress(phase: u32, done: u32, total: u32);
    }
    // SAFETY: the host provides `env.progress` with this signature; it takes three integers and reads no memory of ours.
    unsafe { host_progress(phase, done as u32, total as u32) }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) const fn progress(_phase: u32, _done: usize, _total: usize) {}

// Phase timers and counters per plan (`next.work.prof`); val/same.py excludes timings but compares work counters.
thread_local! { pub(crate) static PROF: std::cell::RefCell<std::collections::BTreeMap<&'static str, f64>> = std::cell::RefCell::default(); }

/// Add `amount` to this plan's counter or timer `name` in `PROF`.
pub(crate) fn prof(name: &'static str, amount: f64) {
    PROF.with(|counters| *counters.borrow_mut().entry(name).or_insert(0.0) += amount);
}
