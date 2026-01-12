#!/usr/bin/env perl

use strict;
use warnings;
use Test::More tests => 4;

BEGIN {
    use_ok('Kreuzberg');
    use_ok('Kreuzberg::FFI');
    use_ok('Kreuzberg::Result');
    use_ok('Kreuzberg::Config');
}

diag("Testing Kreuzberg $Kreuzberg::VERSION, Perl $], $^X");
