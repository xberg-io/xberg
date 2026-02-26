# C examples for kreuzberg-ffi
#
# Prerequisites:
#   - kreuzberg-ffi installed (pkg-config --libs kreuzberg works)
#   OR
#   - Set LIBDIR and HEADER_DIR to point at your build artifacts:
#       make LIBDIR=../../target/release HEADER_DIR=../../crates/kreuzberg-ffi

HEADER_DIR ?= ../../crates/kreuzberg-ffi
LIBDIR ?= ../../target/release
LIBNAME = kreuzberg_ffi

CC ?= cc
CFLAGS = -Wall -Wextra -I$(HEADER_DIR)

ifeq ($(shell uname),Darwin)
  LDFLAGS = -L$(LIBDIR) -l$(LIBNAME) -framework CoreFoundation -framework Security -lpthread
  RPATH = -Wl,-rpath,$(LIBDIR)
else ifeq ($(OS),Windows_NT)
  LDFLAGS = -L$(LIBDIR) -l$(LIBNAME) -lws2_32 -luserenv -lbcrypt
  RPATH =
else
  LDFLAGS = -L$(LIBDIR) -l$(LIBNAME) -lpthread -ldl -lm
  RPATH = -Wl,-rpath,$(LIBDIR)
endif

EXAMPLES = basic_extraction batch_extraction

.PHONY: all clean test

all: $(EXAMPLES)

basic_extraction: basic_extraction.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

batch_extraction: batch_extraction.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test: all
	@for e in $(EXAMPLES); do echo "Running $$e..."; ./$$e || exit 1; done

clean:
	rm -f $(EXAMPLES)
