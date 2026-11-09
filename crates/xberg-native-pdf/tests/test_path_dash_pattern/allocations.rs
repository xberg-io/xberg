use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use super::document_with_resource_objects;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static ALLOCATED_BYTES: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;

fn record_allocation(size: usize) {
    if COUNTING.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        let _ = ALLOCATED_BYTES.try_with(|count| count.set(count.get() + size));
    }
}

// SAFETY: Every allocation operation delegates unchanged to System; counters do not allocate.
#[allow(
    unsafe_code,
    reason = "measuring real allocations requires delegating the GlobalAlloc interface"
)]
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        // SAFETY: The caller supplies the allocation layout required by GlobalAlloc.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: The allocation is owned by System and its original layout is forwarded.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation(size);
        // SAFETY: The caller's allocation and requested size are forwarded unchanged.
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

struct CountingScope;

pub(super) fn measure<T>(action: impl FnOnce() -> T) -> (T, usize, usize) {
    ALLOCATIONS.set(0);
    ALLOCATED_BYTES.set(0);
    COUNTING.set(true);
    let scope = CountingScope;
    let output = action();
    drop(scope);
    (output, ALLOCATIONS.get(), ALLOCATED_BYTES.get())
}

impl Drop for CountingScope {
    fn drop(&mut self) {
        COUNTING.set(false);
    }
}

#[derive(Clone, Copy, Debug)]
enum StateStorage {
    Inline,
    IndirectDictionary,
    IndirectState,
}

fn extraction_allocations(states: usize, storage: StateStorage) -> usize {
    const GS_OPERATORS: usize = 128;
    let states = (0..states)
        .map(|index| format!("/S{index} << /CA 1 >>"))
        .collect::<Vec<_>>()
        .join(" ");
    let content = format!("{}0 700 m 300 700 l S", "/S0 gs ".repeat(GS_OPERATORS));
    let (resources, objects) = match storage {
        StateStorage::Inline => (format!("/ExtGState << {states} >>"), vec![]),
        StateStorage::IndirectDictionary => ("/ExtGState 5 0 R".to_owned(), vec![format!("<< {states} >>")]),
        // Unknown keys are legal extension data and still get deep-cloned by load_object.
        StateStorage::IndirectState => (
            "/ExtGState << /S0 5 0 R >>".to_owned(),
            vec![format!("<< /CA 1 /Unused << {states} >> >>")],
        ),
    };
    let objects = objects.iter().map(String::as_str).collect::<Vec<_>>();
    let document = document_with_resource_objects(content.as_bytes(), &resources, &objects);
    assert_eq!(document.extract_paths(0).unwrap().len(), 1);
    ALLOCATIONS.set(0);
    COUNTING.set(true);
    let scope = CountingScope;
    let paths = document.extract_paths(0).expect("real repeated gs extraction");
    drop(scope);
    assert_eq!(paths.len(), 1);
    ALLOCATIONS.get()
}

fn assert_bounded_state_allocations(storage: StateStorage) {
    const EXTRA_STATES: usize = 63;
    // One-time resource copies can grow with states; 128 gs operators must not multiply them.
    const ALLOCATIONS_PER_EXTRA_STATE: usize = 16;
    let small = extraction_allocations(1, storage);
    let large = extraction_allocations(1 + EXTRA_STATES, storage);
    assert!(
        large <= small + EXTRA_STATES * ALLOCATIONS_PER_EXTRA_STATE,
        "{storage:?} copies multiplied by gs count: one state={small}, 64 states={large}"
    );
}

#[test]
fn should_not_clone_all_inline_states_for_each_gs_operator() {
    assert_bounded_state_allocations(StateStorage::Inline);
}

#[test]
fn should_not_clone_indirect_extgstate_dictionary_for_each_gs_operator() {
    assert_bounded_state_allocations(StateStorage::IndirectDictionary);
}

#[test]
fn should_not_clone_indirect_named_state_for_each_gs_operator() {
    assert_bounded_state_allocations(StateStorage::IndirectState);
}

/// Every `/S{i}` is used exactly once, so each name is a cache miss in its resource scope.
fn distinct_name_allocations(states: usize, storage: StateStorage) -> usize {
    let dictionary = (0..states)
        .map(|index| format!("/S{index} << /CA 1 >>"))
        .collect::<Vec<_>>()
        .join(" ");
    let content = format!(
        "{}0 700 m 300 700 l S",
        (0..states).map(|index| format!("/S{index} gs ")).collect::<String>()
    );
    let (resources, objects) = match storage {
        StateStorage::Inline => (format!("/ExtGState << {dictionary} >>"), vec![]),
        _ => ("/ExtGState 5 0 R".to_owned(), vec![format!("<< {dictionary} >>")]),
    };
    let objects = objects.iter().map(String::as_str).collect::<Vec<_>>();
    let document = document_with_resource_objects(content.as_bytes(), &resources, &objects);
    ALLOCATIONS.set(0);
    COUNTING.set(true);
    let scope = CountingScope;
    let paths = document.extract_paths(0).expect("distinct gs extraction");
    drop(scope);
    assert_eq!(paths.len(), 1);
    ALLOCATIONS.get()
}

#[test]
fn should_resolve_indirect_extgstate_once_for_many_distinct_names() {
    const NAMES: usize = 2000;
    let small = distinct_name_allocations(NAMES, StateStorage::IndirectDictionary);
    let large = distinct_name_allocations(2 * NAMES, StateStorage::IndirectDictionary);
    let inline = distinct_name_allocations(2 * NAMES, StateStorage::Inline);
    assert!(
        large < small * 3,
        "allocations grew superlinearly: N={small}, 2N={large}"
    );
    assert!(
        large <= inline * 2,
        "indirect dictionary costs far more than inline: indirect={large}, inline={inline}"
    );
}

#[test]
fn should_not_clone_all_inline_states_for_each_distinct_gs_name() {
    const NAMES: usize = 500;
    // Every name misses the per-name cache. A copy of the dictionary per lookup makes the cost grow
    // with the square of the name count, so doubling the names would about quadruple the allocations.
    let small = distinct_name_allocations(NAMES, StateStorage::Inline);
    let large = distinct_name_allocations(2 * NAMES, StateStorage::Inline);
    assert!(
        large < small * 3,
        "dictionary copied per distinct gs name: N={small}, 2N={large}"
    );
}
