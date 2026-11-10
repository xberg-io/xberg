use super::{ExtractionConfig, proof_limits};
use std::collections::BTreeMap;

const MAX_RUN_PAGES: usize = 8;
const MAX_RUN_CROPS: usize = 64;
const MAX_RUN_WORK_BYTES: usize = 128 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Budget {
    state: parking_lot::Mutex<State>,
}
#[derive(Default)]
struct State {
    pages: BTreeMap<usize, bool>,
    crops: usize,
    spent: usize,
    limits: Option<Limits>,
    limited: bool,
}
#[derive(Clone, Copy)]
struct Limits {
    pages: usize,
    crops: usize,
    bytes: usize,
}
impl State {
    fn limits(&mut self, config: &ExtractionConfig) -> Limits {
        *self.limits.get_or_insert_with(|| {
            let limits = proof_limits(config);
            Limits {
                pages: MAX_RUN_PAGES.min(limits.max_iterations),
                crops: MAX_RUN_CROPS.min(limits.max_iterations),
                bytes: MAX_RUN_WORK_BYTES.min(limits.max_content_size),
            }
        })
    }
    fn reserve(&mut self, index: usize, config: &ExtractionConfig) -> bool {
        let limits = self.limits(config);
        if self.pages.contains_key(&index) {
            return true;
        }
        if self.pages.len() >= limits.pages || self.spent >= limits.bytes {
            self.limited = true;
            return false;
        }
        self.pages.insert(index, false);
        true
    }
}
impl Budget {
    pub(super) fn reserve_page(&self, index: usize, config: &ExtractionConfig) -> bool {
        self.state.lock().reserve(index, config)
    }
    pub(super) fn begin_page(&self, index: usize, config: &ExtractionConfig) -> bool {
        let mut state = self.state.lock();
        if !state.reserve(index, config) {
            return false;
        }
        let started = state.pages.get_mut(&index).expect("reserved page");
        if *started {
            state.limited = true;
            return false;
        }
        *started = true;
        true
    }
    pub(super) fn remaining(&self, config: &ExtractionConfig) -> usize {
        let mut state = self.state.lock();
        let limits = state.limits(config);
        limits.bytes.saturating_sub(state.spent)
    }
    pub(super) fn reserve_work(&self, config: &ExtractionConfig) -> Option<usize> {
        let mut state = self.state.lock();
        let limits = state.limits(config);
        let allowance = limits.bytes.saturating_sub(state.spent);
        if allowance == 0 {
            state.limited = true;
            return None;
        }
        state.spent = limits.bytes;
        Some(allowance)
    }
    pub(super) fn settle_work(&self, reserved: usize, measured: usize) -> bool {
        let mut state = self.state.lock();
        let Some(unused) = reserved.checked_sub(measured) else {
            state.limited = true;
            return false;
        };
        state.spent = state.spent.saturating_sub(unused);
        true
    }
    pub(super) fn retain_failed_work(&self) {
        self.state.lock().limited = true;
    }
    pub(super) fn charge(&self, bytes: usize, config: &ExtractionConfig) -> bool {
        let mut state = self.state.lock();
        let limits = state.limits(config);
        let Some(spent) = state.spent.checked_add(bytes).filter(|spent| *spent <= limits.bytes) else {
            state.limited = true;
            return false;
        };
        state.spent = spent;
        true
    }
    pub(super) fn crop_available(&self, config: &ExtractionConfig) -> bool {
        let mut state = self.state.lock();
        let limits = state.limits(config);
        if state.crops >= limits.crops {
            state.limited = true;
            return false;
        }
        true
    }
    pub(super) fn take_crop(&self, config: &ExtractionConfig) -> bool {
        let mut state = self.state.lock();
        let limits = state.limits(config);
        if state.crops >= limits.crops {
            state.limited = true;
            return false;
        }
        state.crops += 1;
        true
    }
    pub(super) fn limited(&self) -> bool {
        self.state.lock().limited
    }
}
