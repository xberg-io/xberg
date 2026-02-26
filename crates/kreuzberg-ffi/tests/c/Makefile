# C test suite for kreuzberg-ffi
# Usage: make [LIBDIR=path] [INCDIR=path]

HEADER_DIR ?= ../..
LIBDIR ?= ../../../../target/debug
LIBNAME = kreuzberg_ffi

CC ?= cc
CFLAGS = -Wall -Wextra -Werror -I$(HEADER_DIR)

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

TESTS = test_version test_error test_extraction test_mime \
	test_config test_config_builder test_html_options test_validation test_string_intern \
	test_result_pool test_result_inspect test_plugins test_batch test_error_extended

.PHONY: all clean test

all: $(TESTS)

test_version: test_version.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_error: test_error.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_extraction: test_extraction.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_mime: test_mime.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_config: test_config.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_config_builder: test_config_builder.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_html_options: test_html_options.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_validation: test_validation.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_string_intern: test_string_intern.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_result_pool: test_result_pool.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_result_inspect: test_result_inspect.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_plugins: test_plugins.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_batch: test_batch.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

test_error_extended: test_error_extended.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS) $(RPATH)

clean:
	rm -f $(TESTS)
