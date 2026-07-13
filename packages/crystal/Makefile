# Xberg Crystal — build the FFI library and Crystal bindings
#
# Usage:
#   make              Build FFI + Crystal example
#   make crystal      Build the Crystal library
#   make example      Build the example binary
#   make run          Build and run the example
#   make install      Install the shared library to /usr/local
#   make clean        Remove build artifacts
#
# Prerequisites:
#   - Rust toolchain (cargo, rustc)
#   - Crystal compiler >= 1.0.0
#   - System libraries: libheif >= 1.21 (for HEIF image support)
#     (PKG_CONFIG_PATH=/usr/local/lib/pkgconfig if installed in /usr/local)

ALEF     ?= alef
CARGO    ?= cargo
CRYSTAL  ?= crystal
REPO     ?= $(abspath $(dir $(lastword $(MAKEFILE_LIST)))../..)

FFI_LIB  := $(REPO)/target/release/libxberg_ffi.a
CR_BIN   := $(REPO)/packages/crystal/bin/xberg
EXAMPLE  := $(REPO)/packages/crystal/bin/example

# Shared library path for dynamic linking with RPATH
FFI_DIR  := $(REPO)/target/release
LINKFLAGS := -L$(FFI_DIR) -Wl,-rpath,$(FFI_DIR)

.PHONY: all ffi crystal example run install clean

all: example

$(FFI_LIB):
	cd $(REPO) && $(ALEF) generate --lang ffi
	cd $(REPO) && PKG_CONFIG_PATH=/usr/local/lib/pkgconfig $(CARGO) build -p xberg-ffi --release

ffi: $(FFI_LIB)

$(CR_BIN): $(FFI_LIB)
	cd $(REPO) && $(ALEF) generate --lang crystal
	cd $(REPO)/packages/crystal && $(CRYSTAL) build src/xberg.cr -o bin/xberg --link-flags="$(LINKFLAGS)"

crystal: $(CR_BIN)

$(EXAMPLE): $(FFI_LIB)
	cd $(REPO) && $(ALEF) generate --lang crystal
	cd $(REPO)/packages/crystal && $(CRYSTAL) build examples/example.cr -o bin/example --link-flags="$(LINKFLAGS)"

example: crystal
	$(CRYSTAL) run examples/example.cr --link-flags="$(LINKFLAGS)"

run: example

install: $(FFI_LIB)
	cp $(FFI_DIR)/libxberg_ffi.so /usr/local/lib/
	cp $(FFI_DIR)/libxberg_ffi.a /usr/local/lib/
	@mkdir -p /usr/local/lib/pkgconfig
	@echo "prefix=/usr/local" > /usr/local/lib/pkgconfig/xberg_ffi.pc
	@echo "libdir=/usr/local/lib" >> /usr/local/lib/pkgconfig/xberg_ffi.pc
	@echo "" >> /usr/local/lib/pkgconfig/xberg_ffi.pc
	@echo "Name: xberg_ffi" >> /usr/local/lib/pkgconfig/xberg_ffi.pc
	@echo "Description: C FFI bindings for Xberg" >> /usr/local/lib/pkgconfig/xberg_ffi.pc
	@echo "Version: 1.0.0-rc.14" >> /usr/local/lib/pkgconfig/xberg_ffi.pc
	@echo "Libs: -L\$${libdir} -lxberg_ffi" >> /usr/local/lib/pkgconfig/xberg_ffi.pc
	@ldconfig 2>/dev/null || true
	@echo "Installed xberg_ffi to /usr/local"

clean:
	rm -f $(CR_BIN) $(EXAMPLE)
	cd $(REPO) && $(CARGO) clean -p xberg-ffi --release 2>/dev/null || true
