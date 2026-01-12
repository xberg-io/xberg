#!/usr/bin/env perl

use strict;
use warnings;
use Test::More;
use JSON::PP qw(encode_json);

use_ok('Kreuzberg::Result');

# Test basic result creation
subtest 'basic result creation' => sub {
    my $result = Kreuzberg::Result->new(
        content   => 'Hello, World!',
        mime_type => 'text/plain',
        language  => 'en',
        success   => 1,
    );

    ok( $result, 'Result object created' );
    isa_ok( $result, 'Kreuzberg::Result' );

    is( $result->content,   'Hello, World!', 'Content correct' );
    is( $result->mime_type, 'text/plain',    'MIME type correct' );
    is( $result->language,  'en',            'Language correct' );
    ok( $result->success, 'Success is true' );
};

# Test result with metadata
subtest 'result with metadata' => sub {
    my $metadata = { author => 'John Doe', title => 'Test Document' };

    my $result = Kreuzberg::Result->new(
        content       => 'Test content',
        mime_type     => 'application/pdf',
        metadata_json => encode_json($metadata),
    );

    my $got_metadata = $result->metadata;
    ok( $got_metadata, 'Metadata retrieved' );
    is( $got_metadata->{author}, 'John Doe',      'Author correct' );
    is( $got_metadata->{title},  'Test Document', 'Title correct' );
};

# Test result with tables
subtest 'result with tables' => sub {
    my $tables = [
        { rows => [ [ 'a', 'b' ], [ 'c', 'd' ] ] },
        { rows => [ [ '1', '2' ], [ '3', '4' ] ] },
    ];

    my $result = Kreuzberg::Result->new(
        content     => 'Content with tables',
        mime_type   => 'text/html',
        tables_json => encode_json($tables),
    );

    my $got_tables = $result->tables;
    ok( $got_tables, 'Tables retrieved' );
    is( scalar(@$got_tables), 2, 'Two tables' );
};

# Test result with chunks
subtest 'result with chunks' => sub {
    my $chunks = [
        { text => 'Chunk 1', start => 0,   end => 100 },
        { text => 'Chunk 2', start => 100, end => 200 },
    ];

    my $result = Kreuzberg::Result->new(
        content     => 'Chunked content',
        mime_type   => 'text/plain',
        chunks_json => encode_json($chunks),
    );

    my $got_chunks = $result->chunks;
    ok( $got_chunks, 'Chunks retrieved' );
    is( scalar(@$got_chunks),   2,         'Two chunks' );
    is( $got_chunks->[0]{text}, 'Chunk 1', 'First chunk correct' );
};

# Test to_hash
subtest 'to_hash' => sub {
    my $result = Kreuzberg::Result->new(
        content   => 'Test',
        mime_type => 'text/plain',
        language  => 'en',
        success   => 1,
    );

    my $hash = $result->to_hash;
    ok( $hash, 'Hash retrieved' );
    is( $hash->{content},   'Test',       'Content in hash' );
    is( $hash->{mime_type}, 'text/plain', 'MIME type in hash' );
    is( $hash->{success},   1,            'Success in hash' );
};

# Test empty/undefined fields
subtest 'empty fields' => sub {
    my $result = Kreuzberg::Result->new(
        content   => '',
        mime_type => 'text/plain',
    );

    is( $result->content,  '',    'Empty content works' );
    is( $result->language, undef, 'Undefined language is undef' );
    is_deeply( $result->tables,   [], 'No tables returns empty array' );
    is_deeply( $result->metadata, {}, 'No metadata returns empty hash' );
};

# Test detected_languages
subtest 'detected languages' => sub {
    my $languages = [
        { language => 'en', confidence => 0.9 },
        { language => 'de', confidence => 0.1 },
    ];

    my $result = Kreuzberg::Result->new(
        content                 => 'Multilingual content',
        mime_type               => 'text/plain',
        detected_languages_json => encode_json($languages),
    );

    my $got_languages = $result->detected_languages;
    ok( $got_languages, 'Languages retrieved' );
    is( scalar(@$got_languages),       2,    'Two languages detected' );
    is( $got_languages->[0]{language}, 'en', 'First language is English' );
};

# Test pages
subtest 'pages' => sub {
    my $pages = [
        { number => 1, content => 'Page 1 content' },
        { number => 2, content => 'Page 2 content' },
    ];

    my $result = Kreuzberg::Result->new(
        content    => 'Multi-page document',
        mime_type  => 'application/pdf',
        pages_json => encode_json($pages),
    );

    my $got_pages = $result->pages;
    ok( $got_pages, 'Pages retrieved' );
    is( scalar(@$got_pages),     2, 'Two pages' );
    is( $got_pages->[0]{number}, 1, 'First page number correct' );
};

done_testing();
