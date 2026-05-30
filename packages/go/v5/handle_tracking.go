package kreuzberg

import (
	"runtime/cgo"
	"sync"
)

// handleRegistry tracks cgo.Handles by name to ensure proper cleanup on unregister.
// Without this, unregistered plugins can cause use-after-free crashes when Rust
// still holds vtable pointers and tries to invoke callbacks on deleted handles.
type handleRegistry struct {
	mu      sync.Mutex
	handles map[string]cgo.Handle
}

var (
	documentExtractorRegistry = &handleRegistry{handles: make(map[string]cgo.Handle)}
	ocrBackendRegistry        = &handleRegistry{handles: make(map[string]cgo.Handle)}
	embeddingBackendRegistry  = &handleRegistry{handles: make(map[string]cgo.Handle)}
	postProcessorRegistry     = &handleRegistry{handles: make(map[string]cgo.Handle)}
	rendererRegistry          = &handleRegistry{handles: make(map[string]cgo.Handle)}
	validatorRegistry         = &handleRegistry{handles: make(map[string]cgo.Handle)}
)

// store adds a handle to the registry, keyed by name.
func (reg *handleRegistry) store(name string, handle cgo.Handle) {
	reg.mu.Lock()
	defer reg.mu.Unlock()
	reg.handles[name] = handle
}

// delete removes and deletes a handle from the registry by name.
// Returns true if the handle existed and was deleted.
func (reg *handleRegistry) delete(name string) bool {
	reg.mu.Lock()
	defer reg.mu.Unlock()

	if handle, ok := reg.handles[name]; ok {
		delete(reg.handles, name)
		handle.Delete()
		return true
	}
	return false
}

// deleteAllOnError is called on registration failure to clean up the handle.
// Since registration failed, we need to delete it immediately.
func (reg *handleRegistry) deleteAllOnError(handle cgo.Handle) {
	handle.Delete()
}

// clear removes and deletes all handles from the registry.
func (reg *handleRegistry) clear() {
	reg.mu.Lock()
	defer reg.mu.Unlock()

	for _, handle := range reg.handles {
		handle.Delete()
	}
	reg.handles = make(map[string]cgo.Handle)
}
