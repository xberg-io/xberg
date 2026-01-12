#!/usr/bin/env perl

use strict;
use warnings;
use Test::More;
use JSON::PP qw(decode_json);

use_ok('Kreuzberg::Config');

# Test basic config creation
subtest 'basic config creation' => sub {
    my $config = Kreuzberg::Config->new();
    ok( $config, 'Config object created' );
    isa_ok( $config, 'Kreuzberg::Config' );
};

# Test config with options
subtest 'config with options' => sub {
    my $config = Kreuzberg::Config->new(
        enable_ocr     => 1,
        ocr_language   => 'eng',
        extract_tables => 1,
        chunk_size     => 500,
    );

    is( $config->enable_ocr,     1,     'enable_ocr set correctly' );
    is( $config->ocr_language,   'eng', 'ocr_language set correctly' );
    is( $config->extract_tables, 1,     'extract_tables set correctly' );
    is( $config->chunk_size,     500,   'chunk_size set correctly' );
};

# Test to_json
subtest 'config to_json' => sub {
    my $config = Kreuzberg::Config->new(
        enable_ocr     => 1,
        ocr_language   => 'deu',
        ocr_backend    => 'tesseract',
        extract_tables => 1,
        chunk_size     => 1000,
        chunk_overlap  => 200,
    );

    my $json = $config->to_json;
    ok( $json, 'JSON generated' );

    my $decoded = decode_json($json);
    ok( $decoded, 'JSON is valid' );

    # Check OCR settings
    ok( $decoded->{ocr}, 'OCR config present' );
    is( $decoded->{ocr}{language}, 'deu',       'OCR language correct' );
    is( $decoded->{ocr}{backend},  'tesseract', 'OCR backend correct' );

    # Check table settings
    ok( $decoded->{tables}, 'Tables config present' );

    # Check chunking settings
    ok( $decoded->{chunking}, 'Chunking config present' );
    is( $decoded->{chunking}{max_characters}, 1000, 'Chunk size correct' );
    is( $decoded->{chunking}{overlap},        200,  'Chunk overlap correct' );
};

# Test accessor chaining
subtest 'accessor chaining' => sub {
    my $config = Kreuzberg::Config->new();

    $config->enable_ocr(1)->ocr_language('fra')->extract_tables(1);

    is( $config->enable_ocr,     1,     'enable_ocr set via chaining' );
    is( $config->ocr_language,   'fra', 'ocr_language set via chaining' );
    is( $config->extract_tables, 1,     'extract_tables set via chaining' );
};

# Test PDF config
subtest 'pdf config' => sub {
    my $config = Kreuzberg::Config->new(
        pdf_dpi            => 300,
        pdf_extract_images => 1,
        pdf_password       => 'secret',
    );

    my $json    = $config->to_json;
    my $decoded = decode_json($json);

    ok( $decoded->{pdf}, 'PDF config present' );
    is( $decoded->{pdf}{dpi},      300,      'PDF DPI correct' );
    is( $decoded->{pdf}{password}, 'secret', 'PDF password correct' );
};

# Test keyword extraction config
subtest 'keyword config' => sub {
    my $config = Kreuzberg::Config->new(
        extract_keywords  => 1,
        keyword_algorithm => 'yake',
        max_keywords      => 10,
    );

    my $json    = $config->to_json;
    my $decoded = decode_json($json);

    ok( $decoded->{keywords}, 'Keywords config present' );
    is( $decoded->{keywords}{algorithm}, 'yake', 'Keyword algorithm correct' );
    is( $decoded->{keywords}{max_keywords}, 10,  'Max keywords correct' );
};

done_testing();
