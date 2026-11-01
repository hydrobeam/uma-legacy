//! Host clocks, progress and phase counters.

// The host's clock and progress hook in wasm, imported through wasm-bindgen's glue
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    /// `performance.now()`: the clock the worker times its jobs by too
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;
    /// `solverProgress(phase, done, total)`, a function the worker puts on its global scope before it makes the
    /// `Solver` (src/worker.ts)
    #[wasm_bindgen(js_name = solverProgress)]
    fn host_progress(phase: u32, done: u32, total: u32);
}

/// Elapsed wall-clock ms: the host's `performance.now()` in wasm, else since the first native call.
#[cfg(target_arch = "wasm32")]
pub(crate) fn now_ms() -> f64 {
    performance_now()
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn now_ms() -> f64 {
    static FIRST_CALL: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    FIRST_CALL.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
}

/// the plan's progress for the page's status line, at its phase boundaries (`plan::index_rule`): phase, k of n. In
/// wasm the host's `solverProgress` (the worker posts it to the page while the plan runs); elsewhere nothing
#[cfg(target_arch = "wasm32")]
pub(crate) fn progress(phase: u32, done: usize, total: usize) {
    host_progress(phase, done as u32, total as u32);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) const fn progress(_phase: u32, _done: usize, _total: usize) {}

// Phase timers and counters per plan (`next.work.prof`); val/same.py excludes timings but compares work counters.
thread_local! { pub(crate) static PROF: std::cell::RefCell<std::collections::BTreeMap<&'static str, f64>> = std::cell::RefCell::default(); }

/// Add `amount` to this plan's counter or timer `name` in `PROF`.
pub(crate) fn prof(name: &'static str, amount: f64) {
    PROF.with(|counters| *counters.borrow_mut().entry(name).or_insert(0.0) += amount);
}
