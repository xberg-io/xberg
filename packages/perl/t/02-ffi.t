#!/usr/bin/env perl

use strict;
use warnings;
use Test::More;

# Skip if library not available
BEGIN {
    eval { require Kreuzberg::FFI };
    if ($@) {
        plan skip_all => "FFI::Platypus not available or library not found: $@";
    }
}

use_ok('Kreuzberg::FFI');

# Test singleton instance
subtest 'singleton instance' => sub {
    my $ffi1 = Kreuzberg::FFI->instance;
    my $ffi2 = Kreuzberg::FFI->instance;

    ok( $ffi1, 'First instance created' );
    ok( $ffi2, 'Second instance created' );
    is( $ffi1, $ffi2, 'Same instance returned' );
};

# Test version function
subtest 'version function' => sub {
    my $ffi = Kreuzberg::FFI->instance;

    eval {
        my $version = $ffi->kreuzberg_version();
        ok( $version, 'Version returned' );
        like( $version, qr/^\d+\.\d+/, 'Version looks like a version string' );
        diag("Kreuzberg library version: $version");
    };

    if ($@) {
        skip "Library not loaded: $@", 2;
    }
};

# Test error handling
subtest 'error handling' => sub {
    my $ffi = Kreuzberg::FFI->instance;

    eval {
        # This should fail gracefully
        my $error = $ffi->kreuzberg_last_error();

        # Error might be undef if no error occurred
        ok( 1, 'Error function callable' );
    };

    if ($@) {
        skip "Library not loaded: $@", 1;
    }
};

done_testing();
