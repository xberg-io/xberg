#!/usr/bin/env perl

use strict;
use warnings;
use Test::More;
use Test::Exception;
use File::Temp qw(tempfile);

# Skip if library not available
BEGIN {
    eval {
        require Kreuzberg;
        require Kreuzberg::FFI;
        Kreuzberg::FFI->instance->kreuzberg_version();
    };
    if ($@) {
        plan skip_all => "Kreuzberg library not available: $@";
    }
}

use_ok('Kreuzberg');

# Test version
subtest 'version' => sub {
    my $version = Kreuzberg::version();
    ok( $version, 'Version returned' );
    like( $version, qr/^\d+\.\d+/, 'Version format correct' );
    diag("Kreuzberg version: $version");
};

# Test extract_file with text file
subtest 'extract text file' => sub {

    # Create a temporary text file
    my ( $fh, $filename ) = tempfile( SUFFIX => '.txt', UNLINK => 1 );
    print $fh "Hello, Kreuzberg!";
    close $fh;

    my $result = Kreuzberg::extract_file($filename);
    ok( $result, 'Result returned' );
    isa_ok( $result, 'Kreuzberg::Result' );
    like( $result->content, qr/Hello/, 'Content extracted' );
    is( $result->mime_type, 'text/plain', 'MIME type detected' );
    ok( $result->success, 'Extraction successful' );
};

# Test extract_bytes
subtest 'extract bytes' => sub {
    my $text = "This is test content for byte extraction.";

    my $result = Kreuzberg::extract_bytes( $text, 'text/plain' );
    ok( $result, 'Result returned' );
    like( $result->content, qr/test content/, 'Content extracted from bytes' );
};

# Test extract_file with config
subtest 'extract with config' => sub {
    my ( $fh, $filename ) = tempfile( SUFFIX => '.txt', UNLINK => 1 );
    print $fh "Configuration test content.";
    close $fh;

    my $config = Kreuzberg::Config->new( detect_language => 1, );

    my $result = Kreuzberg::extract_file( $filename, $config );
    ok( $result,          'Result returned with config' );
    ok( $result->success, 'Extraction successful with config' );
};

# Test detect_mime_type
subtest 'detect MIME type' => sub {
    my ( $fh, $filename ) = tempfile( SUFFIX => '.txt', UNLINK => 1 );
    print $fh "MIME detection test";
    close $fh;

    my $mime = Kreuzberg::detect_mime_type($filename);
    ok( $mime, 'MIME type detected' );
    is( $mime, 'text/plain', 'Correct MIME type for .txt' );
};

# Test validate_mime_type
subtest 'validate MIME type' => sub {
    ok( Kreuzberg::validate_mime_type('text/plain'), 'text/plain is valid' );
    ok( Kreuzberg::validate_mime_type('application/pdf'),
        'application/pdf is valid' );
};

# Test detect_mime_type_from_bytes
subtest 'detect MIME type from bytes' => sub {
    my $text_bytes = "Hello, this is plain text content";
    my $mime = Kreuzberg::detect_mime_type_from_bytes($text_bytes);
    ok( $mime, 'MIME type detected from bytes' );
    is( $mime, 'text/plain', 'Correct MIME type for text bytes' );
};

# Test get_extensions_for_mime
subtest 'get extensions for MIME type' => sub {
    my $ext = Kreuzberg::get_extensions_for_mime('application/pdf');
    ok( defined $ext, 'Extension returned for application/pdf' );
    like( $ext, qr/pdf/i, 'Extension contains pdf' );

    my $txt_ext = Kreuzberg::get_extensions_for_mime('text/plain');
    ok( defined $txt_ext, 'Extension returned for text/plain' );
    like( $txt_ext, qr/txt/i, 'Extension contains txt' );
};

# Test error handling - nonexistent file
subtest 'error handling - nonexistent file' => sub {
    throws_ok {
        Kreuzberg::extract_file('/nonexistent/file/path.txt');
    }
    qr/does not exist|failed/i, 'Dies on nonexistent file';
};

# Test error handling - missing arguments
subtest 'error handling - missing arguments' => sub {
    throws_ok {
        Kreuzberg::extract_file();
    }
    qr/required|Too few arguments/i, 'Dies on missing path';

    throws_ok {
        Kreuzberg::extract_bytes( undef, 'text/plain' );
    }
    qr/required/i, 'Dies on missing bytes';

    throws_ok {
        Kreuzberg::extract_bytes( "data", undef );
    }
    qr/required/i, 'Dies on missing MIME type';
};

# Test list_ocr_backends
subtest 'list OCR backends' => sub {
    my @backends = Kreuzberg::list_ocr_backends();
    ok( defined \@backends, 'Backends list returned' );

    # May be empty if no OCR backends installed, that's OK
};

# Test batch extraction
subtest 'batch extraction' => sub {
    my ( $fh1, $file1 ) = tempfile( SUFFIX => '.txt', UNLINK => 1 );
    print $fh1 "First file content";
    close $fh1;

    my ( $fh2, $file2 ) = tempfile( SUFFIX => '.txt', UNLINK => 1 );
    print $fh2 "Second file content";
    close $fh2;

    my @results = Kreuzberg::batch_extract_files( [ $file1, $file2 ] );
    is( scalar(@results), 2, 'Two results returned' );

    for my $result (@results) {
        ok( $result,          'Result exists' );
        ok( $result->success, 'Extraction successful' );
    }
};

done_testing();
