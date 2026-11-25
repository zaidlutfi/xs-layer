//! Bridge between layer-owned fence signals and an emulated monitor/wait pair.
//!
//! A ticket keeps the wake word alive independently of the fence. The title's
//! address is a registry key and is never dereferenced by the exception handler.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock, Weak};
use windows_sys::Win32::Foundation::{GetLastError, SetLastError};
use windows_sys::Win32::System::Threading::{WaitOnAddress, WakeByAddressAll};

/// Explicit opt-in while game timing on Intel is being measured.
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("VICTORY_MONITOR_FENCE_WAIT").is_some_and(|v| v == "1"))
}

type Registry = RwLock<HashMap<usize, Weak<AtomicU64>>>;
static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// A registered GPU fence's wake word. Owned by the fence.
pub struct Signal {
    address: usize,
    generation: Arc<AtomicU64>,
}

impl Signal {
    /// Registers the address exposed to the title for the lifetime of this object.
    pub fn register(address: usize) -> Self {
        let generation = Arc::new(AtomicU64::new(0));
        REGISTRY
            .get_or_init(RwLock::default)
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(address, Arc::downgrade(&generation));
        Self { address, generation }
    }

    /// Call after publishing the completed value.
    pub fn notify(&self) {
        self.generation.fetch_add(1, Ordering::Release);
        // SAFETY: this is the aligned, live word also held by every ticket.
        unsafe { WakeByAddressAll(self.generation.as_ptr().cast()) };
    }
}

impl Drop for Signal {
    fn drop(&mut self) {
        let mut registry = REGISTRY.get().unwrap().write().unwrap_or_else(|e| e.into_inner());
        if registry
            .get(&self.address)
            .is_some_and(|entry| entry.as_ptr() == Arc::as_ptr(&self.generation))
        {
            registry.remove(&self.address);
        }
        drop(registry);
        self.notify();
    }
}

/// One monitor snapshot. A signal between capture and waiting is visible.
pub struct Ticket {
    generation: Arc<AtomicU64>,
    observed: u64,
}

impl Ticket {
    /// Captures a known fence. Contention in the exception handler falls back
    /// to the existing spin hint.
    pub fn arm(address: usize) -> Option<Self> {
        let generation = REGISTRY.get()?.try_read().ok()?.get(&address)?.upgrade()?;
        let observed = generation.load(Ordering::Acquire);
        Some(Self { generation, observed })
    }

    /// Waits once with a requested 1 ms timeout. The title rechecks the fence.
    /// Returns whether the wake word changed.
    pub fn wait(self) -> bool {
        self.wait_for(1)
    }

    fn wait_for(self, timeout_ms: u32) -> bool {
        if self.generation.load(Ordering::Acquire) == self.observed {
            // SAFETY: both 8-byte words are live and aligned. Preserve the
            // calling thread's Windows error state through the emulated hint.
            unsafe {
                let saved = GetLastError();
                WaitOnAddress(
                    self.generation.as_ptr().cast(),
                    (&raw const self.observed).cast(),
                    size_of::<u64>(),
                    timeout_ms,
                );
                SetLastError(saved);
            }
        }
        self.generation.load(Ordering::Acquire) != self.observed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn tickets_follow_the_fence_lifetime_and_address_reuse() {
        let _serial = TESTS.lock().unwrap();
        let word = 0u64;
        let key = (&raw const word) as usize;
        assert!(Ticket::arm(key).is_none());
        let signal = Signal::register(key);
        let previous = Ticket::arm(key).unwrap();
        drop(signal);
        assert!(Ticket::arm(key).is_none());
        let replacement = Signal::register(key);
        let next = Ticket::arm(key).unwrap();
        assert!(previous.wait());
        // SAFETY: this changes only the current test thread's error value.
        let previous_error = unsafe { GetLastError() };
        unsafe { SetLastError(1234) };
        assert!(!next.wait());
        let restored = unsafe { GetLastError() };
        unsafe { SetLastError(previous_error) };
        assert_eq!(restored, 1234);
        let next = Ticket::arm(key).unwrap();
        replacement.notify();
        assert!(next.wait());
    }

    #[test]
    fn a_signal_reaches_multiple_armed_waiters() {
        let _serial = TESTS.lock().unwrap();
        let word = 0u64;
        let key = (&raw const word) as usize;
        let signal = Signal::register(key);
        std::thread::scope(|scope| {
            let first = Ticket::arm(key).unwrap();
            let second = Ticket::arm(key).unwrap();
            let a = scope.spawn(|| first.wait_for(500));
            let b = scope.spawn(|| second.wait_for(500));
            signal.notify();
            assert!(a.join().unwrap());
            assert!(b.join().unwrap());
        });
    }
}
