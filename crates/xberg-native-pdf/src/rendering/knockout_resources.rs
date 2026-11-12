use crate::error::{Error, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const MAX_WORKING_BYTES: usize = 512 * 1024 * 1024;
const MAX_SURFACE_WORK: u64 = 1024 * 1024 * 1024;
pub(super) const MAX_NESTING_DEPTH: usize = 64;
const MAX_PAINT_OPERATIONS: usize = 1_000_000;

pub(super) struct KnockoutResources {
    limit: usize,
    live: Arc<AtomicUsize>,
    depth: Arc<AtomicUsize>,
    surface_work: u64,
    paint_operations: usize,
    #[cfg(test)]
    paint_operation_limit: usize,
    failure: Option<String>,
    workspace_shortfall: Option<(usize, usize)>,
    work_exhausted: bool,
    degradations: usize,
    native: Arc<AtomicUsize>,
    degraded: Arc<AtomicUsize>,
}

impl Default for KnockoutResources {
    fn default() -> Self {
        Self {
            limit: MAX_WORKING_BYTES,
            live: Arc::new(AtomicUsize::new(0)),
            depth: Arc::new(AtomicUsize::new(0)),
            surface_work: 0,
            paint_operations: 0,
            #[cfg(test)]
            paint_operation_limit: MAX_PAINT_OPERATIONS,
            failure: None,
            workspace_shortfall: None,
            work_exhausted: false,
            degradations: 0,
            native: Arc::new(AtomicUsize::new(0)),
            degraded: Arc::new(AtomicUsize::new(0)),
        }
    }
}

pub(super) struct Reservation {
    live: Arc<AtomicUsize>,
    depth: Arc<AtomicUsize>,
    bytes: usize,
}

impl Drop for Reservation {
    fn drop(&mut self) {
        self.live.fetch_sub(self.bytes, Ordering::Relaxed);
        self.depth.fetch_sub(1, Ordering::Relaxed);
    }
}

pub(super) struct KnockoutReservation {
    _workspace: Reservation,
    native: Arc<AtomicUsize>,
}

impl Drop for KnockoutReservation {
    fn drop(&mut self) {
        self.native.fetch_sub(1, Ordering::Relaxed);
    }
}

pub(super) struct WorkFallback {
    _scope: Reservation,
    degraded: Arc<AtomicUsize>,
}

impl Drop for WorkFallback {
    fn drop(&mut self) {
        self.degraded.fetch_sub(1, Ordering::Relaxed);
    }
}

impl KnockoutResources {
    pub(super) fn reset(&mut self, limit: Option<usize>) {
        self.limit = limit.unwrap_or(MAX_WORKING_BYTES).min(MAX_WORKING_BYTES);
        self.paint_operations = 0;
        self.reset_attempt();
    }

    /// A DPI retry gets fresh raster state but spends the same page's paint budget.
    pub(super) fn reset_attempt(&mut self) {
        self.live.store(0, Ordering::Relaxed);
        self.depth.store(0, Ordering::Relaxed);
        self.surface_work = 0;
        self.failure = None;
        self.workspace_shortfall = None;
        self.work_exhausted = false;
        self.degradations = 0;
        self.native.store(0, Ordering::Relaxed);
        self.degraded.store(0, Ordering::Relaxed);
    }

    pub(super) fn canvas(&mut self, bytes: usize) {
        self.live.store(bytes, Ordering::Relaxed);
    }

    pub(super) fn active(&self) -> bool {
        self.depth.load(Ordering::Relaxed) > 0
    }

    pub(super) fn workspace_shortfall(&self) -> Option<(usize, usize)> {
        self.workspace_shortfall
    }

    pub(super) fn retained_bytes(&self) -> u64 {
        self.live.load(Ordering::Relaxed) as u64
    }

    fn refuse<T>(&mut self, reason: &str) -> Result<T> {
        let message = format!("PDF knockout {reason} exceeds render working limits");
        self.failure = Some(message.clone());
        Err(Error::InvalidOperation(message))
    }

    pub(super) fn check(&self) -> Result<()> {
        if let Some(message) = &self.failure {
            return Err(Error::InvalidOperation(message.clone()));
        }
        if self.work_exhausted {
            return Err(Error::InvalidOperation(
                "PDF knockout surface work exceeds render working limits".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn work_exhausted(&self) -> bool {
        self.work_exhausted && self.failure.is_none()
    }

    pub(super) fn degradations(&self) -> usize {
        self.degradations
    }

    pub(super) fn fallback(&mut self) -> Result<WorkFallback> {
        if self.failure.is_some() {
            self.check()?;
        }
        // Only the handled work refusal is cleared; fatal resource refusals stay latched. ~keep
        self.work_exhausted = false;
        let scope = self.reserve(Some(0))?;
        if self.degraded.fetch_add(1, Ordering::Relaxed) == 0 {
            self.degradations = self.degradations.saturating_add(1);
        }
        Ok(WorkFallback {
            _scope: scope,
            degraded: Arc::clone(&self.degraded),
        })
    }

    pub(super) fn reserve(&mut self, required: Option<usize>) -> Result<Reservation> {
        self.check()?;
        if self.depth.load(Ordering::Relaxed) >= MAX_NESTING_DEPTH {
            return self.refuse("nesting");
        }
        let Some(bytes) = required else {
            return self.refuse("workspace arithmetic");
        };
        let Some(next) = self.live.load(Ordering::Relaxed).checked_add(bytes) else {
            return self.refuse("workspace arithmetic");
        };
        if next > isize::MAX as usize {
            return self.refuse("workspace arithmetic");
        }
        if next > self.limit {
            self.workspace_shortfall = Some((next, self.limit));
            return self.refuse("workspace");
        }
        self.live.fetch_add(bytes, Ordering::Relaxed);
        self.depth.fetch_add(1, Ordering::Relaxed);
        Ok(Reservation {
            live: Arc::clone(&self.live),
            depth: Arc::clone(&self.depth),
            bytes,
        })
    }

    pub(super) fn knockout(
        &mut self,
        pixels: usize,
        rgba_bytes: usize,
        sidecar_bytes: usize,
        spot_count: usize,
        paints: usize,
        surfaces_in_pass: usize,
    ) -> Result<Option<KnockoutReservation>> {
        if self.failure.is_some() {
            self.check()?;
        }
        if self.degraded.load(Ordering::Relaxed) > 0 || self.surface_work > MAX_SURFACE_WORK {
            return Ok(None);
        }
        let work = (pixels as u64)
            .checked_mul(paints as u64)
            .and_then(|value| value.checked_add(self.surface_work));
        let Some(work) = work else {
            return self.refuse("surface work arithmetic");
        };
        if work > MAX_SURFACE_WORK {
            return Ok(None);
        }
        // Running the planned paints charges at least `surfaces_in_pass` full surfaces, so a plan
        // whose floor already passes the limit would exhaust the work part-way and render the
        // whole group a second time. Decide that before the pass. ~keep
        let floor = (pixels as u64)
            .checked_mul(surfaces_in_pass as u64)
            .and_then(|pass| pass.checked_add(work));
        if floor.is_none_or(|floor| floor > MAX_SURFACE_WORK) {
            return Ok(None);
        }
        let bytes = rgba_bytes
            .checked_add(sidecar_bytes)
            .and_then(|value| value.checked_mul(2))
            .and_then(|value| shape_bytes(pixels, spot_count)?.checked_mul(2)?.checked_add(value));
        let reservation = self.reserve(bytes)?;
        self.surface_work = work;
        self.native.fetch_add(1, Ordering::Relaxed);
        Ok(Some(KnockoutReservation {
            _workspace: reservation,
            native: Arc::clone(&self.native),
        }))
    }

    pub(super) fn paint(&mut self, pixels: usize) -> Result<()> {
        // Every group counts paints, so nested form fan-out stays bounded; only pixel work is knockout-only. ~keep
        #[cfg(test)]
        let limit = self.paint_operation_limit;
        #[cfg(not(test))]
        let limit = MAX_PAINT_OPERATIONS;
        if self.paint_operations >= limit {
            return self.refuse("paint operations");
        }
        self.paint_operations += 1;
        self.raster(pixels)
    }

    pub(super) fn raster(&mut self, pixels: usize) -> Result<()> {
        self.check()?;
        // Only a native knockout pass is charged: ordinary groups render as they do without knockout. ~keep
        if self.degraded.load(Ordering::Relaxed) > 0 || self.native.load(Ordering::Relaxed) == 0 {
            return Ok(());
        }
        self.surface_work = self.surface_work.saturating_add(pixels as u64);
        self.work_exhausted = self.surface_work > MAX_SURFACE_WORK;
        self.check()
    }

    pub(super) fn group(&mut self, rgba_bytes: usize, sidecar_bytes: Option<usize>) -> Result<Reservation> {
        self.reserve(sidecar_bytes.and_then(|bytes| bytes.checked_add(rgba_bytes)))
    }

    pub(super) fn pattern(&mut self, cell_pixels: usize, page_bytes: usize) -> Result<Reservation> {
        self.reserve(
            cell_pixels
                .checked_mul(9)
                .and_then(|value| value.checked_add(page_bytes)),
        )
    }
}

pub(super) fn shape_bytes(pixels: usize, inks: usize) -> Option<usize> {
    // Allow space for the per-ink map buckets as well as their raster planes. ~keep
    let entry = 4 * (std::mem::size_of::<(usize, Vec<u8>)>() + 1);
    let map_bytes = if inks == 0 {
        0
    } else {
        inks.checked_mul(entry)?.checked_add(32)?
    };
    pixels.checked_mul(inks.checked_add(1)?)?.checked_add(map_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reservations_release_capacity_and_refusals_stay_latched() {
        let mut resources = KnockoutResources::default();
        resources.reset(Some(24));
        resources.canvas(8);
        drop(resources.reserve(Some(16)).expect("exact boundary fits"));
        let reservation = resources.reserve(Some(16)).expect("released bytes can be reused");
        assert!(resources.reserve(Some(1)).is_err());
        assert_eq!(resources.workspace_shortfall(), Some((25, 24)));
        drop(reservation);
        assert!(
            resources.reserve(Some(1)).is_err(),
            "an earlier refusal cannot be hidden by unwind"
        );
        resources.reset(Some(24));
        resources.canvas(8);
        assert_eq!(resources.workspace_shortfall(), None);
        assert!(resources.reserve(Some(16)).is_ok(), "a new page resets the refusal");
    }

    #[test]
    fn nesting_refuses_before_unbounded_recursive_allocations() {
        let mut resources = KnockoutResources::default();
        let mut frames = Vec::new();
        for _ in 0..MAX_NESTING_DEPTH {
            frames.push(resources.reserve(Some(0)).expect("within nesting bound"));
        }
        assert_eq!(frames.len(), 64);
        assert!(resources.reserve(Some(0)).is_err());
        assert_eq!(resources.workspace_shortfall(), None, "resizing cannot fix nesting");
        drop(frames);
        assert_eq!(resources.depth.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn surface_work_overflow_is_refused_before_buffers_are_allocated() {
        let mut resources = KnockoutResources::default();
        assert!(resources.knockout(usize::MAX, 0, 0, 0, usize::MAX, usize::MAX).is_err());
        assert_eq!(resources.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn image_retention_counts_nested_workspaces_until_release() {
        let mut resources = KnockoutResources::default();
        resources.canvas(128);
        let group = resources.group(32, Some(24)).expect("group and paired sidecar capture");
        let image = resources.reserve(Some(16)).expect("nested raster");
        assert_eq!(resources.retained_bytes(), 200);
        drop(image);
        assert_eq!(resources.retained_bytes(), 184);
        drop(group);
        assert_eq!(resources.retained_bytes(), 128);
    }

    #[test]
    fn ordinary_isolated_rgb_group_admits_two_live_surfaces() {
        let rgba_bytes = 4 * 4_960 * 7_016;
        let mut resources = KnockoutResources::default();
        resources.reset(Some(512 * 1024 * 1024));
        resources.canvas(rgba_bytes);
        let group = resources
            .group(rgba_bytes, Some(0))
            .expect("normal group requires only one extra RGBA surface");
        assert_eq!(resources.live.load(Ordering::Relaxed), 2 * rgba_bytes);
        assert!(
            resources.reserve(Some(3 * rgba_bytes)).is_err(),
            "optional scratch must still fit separately"
        );
        drop(group);
    }
}

#[cfg(test)]
#[path = "knockout_work_tests.rs"]
mod work_tests;
