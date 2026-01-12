package Kreuzberg;

use strict;
use warnings;
use v5.20;
use feature 'signatures';
no warnings 'experimental::signatures';

use Carp qw(croak);
use File::Spec;
use JSON::PP qw(decode_json);

use Kreuzberg::FFI;
use Kreuzberg::Result;
use Kreuzberg::Config;
use FFI::Platypus::Buffer qw(scalar_to_buffer);
use FFI::Platypus::Memory qw(strdup free);

our $VERSION = '4.0.1';

=head1 NAME

Kreuzberg - High-performance document intelligence framework

=head1 SYNOPSIS

    use Kreuzberg;

    # Simple file extraction
    my $result = Kreuzberg::extract_file('/path/to/document.pdf');
    print $result->content;

    # With configuration
    my $config = Kreuzberg::Config->new(
        enable_ocr => 1,
        ocr_language => 'eng',
    );
    my $result = Kreuzberg::extract_file('/path/to/image.png', $config);

    # Extract from bytes
    my $bytes = read_file_to_bytes('/path/to/file.docx');
    my $result = Kreuzberg::extract_bytes($bytes, 'application/vnd.openxmlformats-officedocument.wordprocessingml.document');

    # Batch extraction
    my @results = Kreuzberg::batch_extract_files([
        '/path/to/file1.pdf',
        '/path/to/file2.docx',
    ]);

=head1 DESCRIPTION

Kreuzberg is a multi-language document intelligence framework with a high-performance
Rust core. Supports extraction, OCR, chunking, and language detection for 50+ file formats
including PDF, DOCX, PPTX, XLSX, images, and more.

This Perl binding provides access to the Kreuzberg Rust library via FFI.

=head1 FUNCTIONS

=head2 extract_file($path, $config?)

Extract text and metadata from a file.

    my $result = Kreuzberg::extract_file('/path/to/document.pdf');
    my $result = Kreuzberg::extract_file('/path/to/document.pdf', $config);

Returns a L<Kreuzberg::Result> object.

=cut

sub extract_file ( $path, $config = undef ) {
    croak "Path is required"           unless defined $path;
    croak "File does not exist: $path" unless -e $path;

    my $ffi = Kreuzberg::FFI->instance;
    my $result_ptr;

    if ( defined $config ) {
        my $config_json = $config->to_json;
        $result_ptr =
          $ffi->kreuzberg_extract_file_sync_with_config( $path, $config_json );
    }
    else {
        $result_ptr = $ffi->kreuzberg_extract_file_sync($path);
    }

    unless ($result_ptr) {
        my $error = _get_last_error();
        croak "Extraction failed: $error";
    }

    my $result = Kreuzberg::Result->_from_ffi($result_ptr);
    $ffi->kreuzberg_free_result($result_ptr);

    return $result;
}

=head2 extract_bytes($bytes, $mime_type, $config?)

Extract text and metadata from byte data.

    my $result = Kreuzberg::extract_bytes($bytes, 'application/pdf');
    my $result = Kreuzberg::extract_bytes($bytes, 'application/pdf', $config);

Returns a L<Kreuzberg::Result> object.

=cut

sub extract_bytes ( $bytes, $mime_type, $config = undef ) {
    croak "Bytes data is required" unless defined $bytes;
    croak "MIME type is required"  unless defined $mime_type;

    my $ffi = Kreuzberg::FFI->instance;
    my $result_ptr;

    # Convert Perl string to buffer pointer
    my ( $ptr, $size ) = scalar_to_buffer($bytes);

    if ( defined $config ) {
        my $config_json = $config->to_json;
        my $config_ptr  = $ffi->kreuzberg_config_from_json($config_json);
        croak "Failed to create config: " . _get_last_error()
          unless $config_ptr;

        $result_ptr = $ffi->kreuzberg_extract_bytes_sync_with_config( $ptr,
            $size, $mime_type, $config_ptr );
        $ffi->kreuzberg_config_free($config_ptr);
    }
    else {
        $result_ptr =
          $ffi->kreuzberg_extract_bytes_sync( $ptr, $size, $mime_type );
    }

    unless ($result_ptr) {
        my $error = _get_last_error();
        croak "Extraction failed: $error";
    }

    my $result = Kreuzberg::Result->_from_ffi($result_ptr);
    $ffi->kreuzberg_free_result($result_ptr);

    return $result;
}

=head2 batch_extract_files(\@paths, $config?)

Extract text and metadata from multiple files.

    my @results = Kreuzberg::batch_extract_files([
        '/path/to/file1.pdf',
        '/path/to/file2.docx',
    ]);

Returns a list of L<Kreuzberg::Result> objects.

=cut

sub batch_extract_files ( $paths, $config = undef ) {
    croak "Paths array is required"
      unless defined $paths && ref($paths) eq 'ARRAY';
    croak "At least one path is required" unless @$paths;

    my $ffi   = Kreuzberg::FFI->instance;
    my $count = scalar @$paths;

    # Create array of C string pointers
    # We need to allocate C strings and keep them alive during the call
    my @c_strings;
    my @ptrs;
    for my $path (@$paths) {
        # Use FFI::Platypus to allocate a C string
        my $c_str = strdup($path);
        push @c_strings, $c_str;
        push @ptrs,      $c_str;
    }

    # Create a packed array of pointers
    my $ptr_array = pack( 'Q*', @ptrs );
    my ( $array_ptr, $array_size ) = scalar_to_buffer($ptr_array);

    my $config_json = defined $config ? $config->to_json : undef;

    my $batch_ptr =
      $ffi->kreuzberg_batch_extract_files_sync( $array_ptr, $count,
        $config_json );

    # Free the C strings we allocated
    for my $c_str (@c_strings) {
        free($c_str);
    }

    unless ($batch_ptr) {
        my $error = _get_last_error();
        croak "Batch extraction failed: $error";
    }

    my @results = Kreuzberg::Result->_from_batch_ffi( $batch_ptr, $count );
    $ffi->kreuzberg_free_batch_result($batch_ptr);

    return @results;
}

=head2 detect_mime_type($path)

Detect the MIME type of a file.

    my $mime = Kreuzberg::detect_mime_type('/path/to/file.pdf');
    # Returns 'application/pdf'

=cut

sub detect_mime_type ($path) {
    croak "Path is required" unless defined $path;

    my $ffi = Kreuzberg::FFI->instance;
    my $ptr = $ffi->kreuzberg_detect_mime_type_from_path($path);

    unless ($ptr) {
        my $error = _get_last_error();
        croak "MIME detection failed: $error";
    }

    my $result = $ffi->cast_to_string($ptr);
    $ffi->kreuzberg_free_string($ptr);

    return $result;
}

=head2 detect_mime_type_from_bytes($bytes)

Detect the MIME type from byte data.

    my $mime = Kreuzberg::detect_mime_type_from_bytes($bytes);

=cut

sub detect_mime_type_from_bytes ($bytes) {
    croak "Bytes data is required" unless defined $bytes;

    my $ffi = Kreuzberg::FFI->instance;

    # Convert Perl string to buffer pointer
    my ( $buf_ptr, $size ) = scalar_to_buffer($bytes);
    my $ptr = $ffi->kreuzberg_detect_mime_type_from_bytes( $buf_ptr, $size );

    unless ($ptr) {
        my $error = _get_last_error();
        croak "MIME detection failed: $error";
    }

    my $result = $ffi->cast_to_string($ptr);
    $ffi->kreuzberg_free_string($ptr);

    return $result;
}

=head2 version()

Get the version of the Kreuzberg library.

    my $version = Kreuzberg::version();

=cut

sub version {
    my $ffi = Kreuzberg::FFI->instance;
    return $ffi->kreuzberg_version();
}

=head2 list_ocr_backends()

List available OCR backends.

    my @backends = Kreuzberg::list_ocr_backends();

=cut

sub list_ocr_backends {
    my $ffi = Kreuzberg::FFI->instance;
    my $ptr = $ffi->kreuzberg_list_ocr_backends();
    return () unless $ptr;

    my $json = $ffi->cast_to_string($ptr);
    $ffi->kreuzberg_free_string($ptr);

    return () unless $json;
    my $result = decode_json($json);

    return @$result;
}

=head2 validate_mime_type($mime_type)

Validate if a MIME type is supported.

    my $valid = Kreuzberg::validate_mime_type('application/pdf');

=cut

sub validate_mime_type ($mime_type) {
    croak "MIME type is required" unless defined $mime_type;

    my $ffi = Kreuzberg::FFI->instance;
    return $ffi->kreuzberg_validate_mime_type($mime_type) ? 1 : 0;
}

=head2 get_extensions_for_mime($mime_type)

Get file extensions for a MIME type.

    my $extensions = Kreuzberg::get_extensions_for_mime('application/pdf');
    # Returns 'pdf'

=cut

sub get_extensions_for_mime ($mime_type) {
    croak "MIME type is required" unless defined $mime_type;

    my $ffi = Kreuzberg::FFI->instance;
    my $ptr = $ffi->kreuzberg_get_extensions_for_mime($mime_type);

    return undef unless $ptr;
    my $result = $ffi->cast_to_string($ptr);
    $ffi->kreuzberg_free_string($ptr);

    return $result;
}

# Internal helper to get last error
sub _get_last_error {
    my $ffi   = Kreuzberg::FFI->instance;
    my $error = $ffi->kreuzberg_last_error();
    return $error // "Unknown error";
}

1;

__END__

=head1 SUPPORTED FORMATS

Kreuzberg supports 50+ document formats including:

=over 4

=item * PDF documents

=item * Microsoft Office (DOCX, XLSX, PPTX)

=item * OpenDocument (ODT, ODS, ODP)

=item * Images (PNG, JPEG, TIFF, WebP, etc.)

=item * Plain text and markup (TXT, HTML, Markdown, XML)

=item * Email formats (EML, MSG)

=item * E-books (EPUB, MOBI)

=item * And many more...

=back

=head1 DEPENDENCIES

This module requires:

=over 4

=item * L<FFI::Platypus> - For calling the Rust library

=item * L<JSON::PP> - For JSON handling (core module)

=item * The Kreuzberg shared library (libkreuzberg_ffi)

=back

=head1 SEE ALSO

=over 4

=item * L<https://kreuzberg.dev> - Documentation

=item * L<https://github.com/kreuzberg-dev/kreuzberg> - Source code

=back

=head1 AUTHORS

Na'aman Hirschfeld E<lt>nhirschfeld@gmail.comE<gt>

Jason Kiniry E<lt>jason.kiniry@gmail.comE<gt>

=head1 LICENSE

MIT License

=cut
