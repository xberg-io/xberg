package Kreuzberg::FFI;

use strict;
use warnings;
use v5.20;
use feature 'signatures';
no warnings 'experimental::signatures';

use Carp qw(croak);
use File::Spec;
use FFI::Platypus 2.00;
use FFI::Platypus::Memory qw(malloc free strdup memcpy);
use FFI::Platypus::Buffer qw(scalar_to_buffer buffer_to_scalar window);

my $_instance;

=head1 NAME

Kreuzberg::FFI - FFI bindings to the Kreuzberg Rust library

=head1 DESCRIPTION

This module provides low-level FFI bindings to the Kreuzberg Rust library.
It is used internally by L<Kreuzberg> and should not be used directly.

=cut

sub instance {
    return $_instance if $_instance;
    $_instance = __PACKAGE__->new();
    return $_instance;
}

sub new ($class) {
    my $self = bless { _funcs => {} }, $class;
    $self->_init_ffi();
    return $self;
}

sub _find_library ($self) {

    # Search order:
    # 1. KREUZBERG_LIB_PATH environment variable
    # 2. LD_LIBRARY_PATH / DYLD_LIBRARY_PATH
    # 3. System library paths
    # 4. Relative to this module (for development)
    # 5. Common installation paths

    my @search_paths;

    # Environment variable override
    if ( $ENV{KREUZBERG_LIB_PATH} ) {
        return $ENV{KREUZBERG_LIB_PATH} if -f $ENV{KREUZBERG_LIB_PATH};
    }

    # Determine library name based on OS
    my $lib_name;
    if ( $^O eq 'darwin' ) {
        $lib_name = 'libkreuzberg_ffi.dylib';
    }
    elsif ( $^O eq 'MSWin32' ) {
        $lib_name = 'kreuzberg_ffi.dll';
    }
    else {
        $lib_name = 'libkreuzberg_ffi.so';
    }

    # Development paths (relative to this module)
    my $module_dir = __FILE__;
    $module_dir =~ s{[/\\]?lib[/\\]Kreuzberg[/\\]FFI\.pm$}{};
    $module_dir = '.' if $module_dir eq '';

    # Convert to absolute path for macOS security (dlopen rejects relative paths)
    use Cwd qw(abs_path);
    $module_dir = abs_path($module_dir) // $module_dir;

    push @search_paths,

      # Development: built library in target/release
      File::Spec->catfile( $module_dir, '..', '..', 'target', 'release',
        $lib_name ),
      File::Spec->catfile( $module_dir, '..', '..', '..', '..', 'target',
        'release', $lib_name ),

      # Vendored in package
      File::Spec->catfile( $module_dir, 'lib', $lib_name ),
      File::Spec->catfile( $module_dir, $lib_name );

    # System paths
    if ( $^O eq 'darwin' ) {
        push @search_paths,
          "/usr/local/lib/$lib_name",
          "/opt/homebrew/lib/$lib_name",
          "$ENV{HOME}/lib/$lib_name";
    }
    elsif ( $^O eq 'MSWin32' ) {
        push @search_paths,
          "C:\\kreuzberg\\$lib_name",
          "$ENV{USERPROFILE}\\kreuzberg\\$lib_name";
    }
    else {
        push @search_paths,
          "/usr/local/lib/$lib_name",
          "/usr/lib/$lib_name",
          "/usr/lib/x86_64-linux-gnu/$lib_name",
          "/usr/lib/aarch64-linux-gnu/$lib_name",
          "$ENV{HOME}/lib/$lib_name";
    }

    for my $path (@search_paths) {
        if ( -f $path ) {
            # Return absolute path for macOS security
            return abs_path($path) // $path;
        }
    }

    # Try to let FFI::Platypus find it
    return 'kreuzberg_ffi';
}

sub _init_ffi ($self) {
    my $ffi = FFI::Platypus->new( api => 2 );

    # Find and load the library
    my $lib_path = $self->_find_library();
    eval { $ffi->lib($lib_path) };
    if ($@) {
        croak "Failed to load Kreuzberg library from '$lib_path': $@\n"
          . "Please ensure the Kreuzberg FFI library is installed.\n"
          . "You can set KREUZBERG_LIB_PATH to the library location.";
    }

    # Define types
    $ffi->type( 'opaque' => 'CExtractionResult' );
    $ffi->type( 'opaque' => 'CBatchResult' );
    $ffi->type( 'opaque' => 'CConfig' );

    # Store function references for direct calling
    my $funcs = $self->{_funcs};

    # Utility functions
    $funcs->{kreuzberg_version} = $ffi->function( 'kreuzberg_version' => [] => 'string' );
    $funcs->{kreuzberg_last_error} = $ffi->function( 'kreuzberg_last_error' => [] => 'string' );
    $funcs->{kreuzberg_last_error_code} = $ffi->function( 'kreuzberg_last_error_code' => [] => 'int' );
    $funcs->{kreuzberg_free_string} = $ffi->function( 'kreuzberg_free_string' => ['opaque'] => 'void' );
    $funcs->{kreuzberg_free_result} = $ffi->function( 'kreuzberg_free_result' => ['CExtractionResult'] => 'void' );
    $funcs->{kreuzberg_free_batch_result} = $ffi->function( 'kreuzberg_free_batch_result' => ['CBatchResult'] => 'void' );
    $funcs->{kreuzberg_clone_string} = $ffi->function( 'kreuzberg_clone_string' => ['string'] => 'opaque' );

    # Extraction functions
    # Note: The _with_config variants take JSON strings, not CConfig pointers
    $funcs->{kreuzberg_extract_file_sync} = $ffi->function( 'kreuzberg_extract_file_sync' => ['string'] => 'CExtractionResult' );
    $funcs->{kreuzberg_extract_file_sync_with_config} = $ffi->function( 'kreuzberg_extract_file_sync_with_config' => [ 'string', 'string' ] => 'CExtractionResult' );
    $funcs->{kreuzberg_extract_bytes_sync} = $ffi->function( 'kreuzberg_extract_bytes_sync' => [ 'opaque', 'size_t', 'string' ] => 'CExtractionResult' );
    $funcs->{kreuzberg_extract_bytes_sync_with_config} = $ffi->function( 'kreuzberg_extract_bytes_sync_with_config' => [ 'opaque', 'size_t', 'string', 'string' ] => 'CExtractionResult' );

    # Batch extraction - takes array of string pointers and JSON config string
    $funcs->{kreuzberg_batch_extract_files_sync} = $ffi->function( 'kreuzberg_batch_extract_files_sync' => [ 'opaque', 'size_t', 'string' ] => 'CBatchResult' );

    # MIME type functions
    $funcs->{kreuzberg_detect_mime_type} = $ffi->function( 'kreuzberg_detect_mime_type' => [ 'opaque', 'size_t' ] => 'opaque' );
    $funcs->{kreuzberg_detect_mime_type_from_path} = $ffi->function( 'kreuzberg_detect_mime_type_from_path' => ['string'] => 'opaque' );
    $funcs->{kreuzberg_detect_mime_type_from_bytes} = $ffi->function( 'kreuzberg_detect_mime_type_from_bytes' => [ 'opaque', 'size_t' ] => 'opaque' );
    $funcs->{kreuzberg_validate_mime_type} = $ffi->function( 'kreuzberg_validate_mime_type' => ['string'] => 'int' );
    $funcs->{kreuzberg_get_extensions_for_mime} = $ffi->function( 'kreuzberg_get_extensions_for_mime' => ['string'] => 'opaque' );

    # Config functions
    $funcs->{kreuzberg_config_from_json} = $ffi->function( 'kreuzberg_config_from_json' => ['string'] => 'CConfig' );
    $funcs->{kreuzberg_config_to_json} = $ffi->function( 'kreuzberg_config_to_json' => ['CConfig'] => 'opaque' );
    $funcs->{kreuzberg_config_free} = $ffi->function( 'kreuzberg_config_free' => ['CConfig'] => 'void' );
    $funcs->{kreuzberg_config_is_valid} = $ffi->function( 'kreuzberg_config_is_valid' => ['CConfig'] => 'int' );

    # Plugin/backend functions
    $funcs->{kreuzberg_list_ocr_backends} = $ffi->function( 'kreuzberg_list_ocr_backends' => [] => 'opaque' );
    $funcs->{kreuzberg_list_post_processors} = $ffi->function( 'kreuzberg_list_post_processors' => [] => 'opaque' );
    $funcs->{kreuzberg_list_validators} = $ffi->function( 'kreuzberg_list_validators' => [] => 'opaque' );

    # Validation functions
    $funcs->{kreuzberg_validate_language_code} = $ffi->function( 'kreuzberg_validate_language_code' => ['string'] => 'int' );
    $funcs->{kreuzberg_validate_ocr_backend} = $ffi->function( 'kreuzberg_validate_ocr_backend' => ['string'] => 'int' );
    $funcs->{kreuzberg_validate_dpi} = $ffi->function( 'kreuzberg_validate_dpi' => ['int'] => 'int' );
    $funcs->{kreuzberg_validate_confidence} = $ffi->function( 'kreuzberg_validate_confidence' => ['double'] => 'int' );

    $self->{ffi} = $ffi;
}

# Explicit wrapper methods that call the stored function references

sub kreuzberg_version ($self) {
    return $self->{_funcs}{kreuzberg_version}->call();
}

sub kreuzberg_last_error ($self) {
    return $self->{_funcs}{kreuzberg_last_error}->call();
}

sub kreuzberg_last_error_code ($self) {
    return $self->{_funcs}{kreuzberg_last_error_code}->call();
}

sub kreuzberg_free_string ($self, $ptr) {
    return $self->{_funcs}{kreuzberg_free_string}->call($ptr);
}

sub kreuzberg_free_result ($self, $ptr) {
    return $self->{_funcs}{kreuzberg_free_result}->call($ptr);
}

sub kreuzberg_free_batch_result ($self, $ptr) {
    return $self->{_funcs}{kreuzberg_free_batch_result}->call($ptr);
}

sub kreuzberg_clone_string ($self, $str) {
    return $self->{_funcs}{kreuzberg_clone_string}->call($str);
}

sub kreuzberg_extract_file_sync ($self, $path) {
    return $self->{_funcs}{kreuzberg_extract_file_sync}->call($path);
}

sub kreuzberg_extract_file_sync_with_config ($self, $path, $config_json) {
    return $self->{_funcs}{kreuzberg_extract_file_sync_with_config}->call($path, $config_json);
}

sub kreuzberg_extract_bytes_sync ($self, $bytes, $len, $mime) {
    return $self->{_funcs}{kreuzberg_extract_bytes_sync}->call($bytes, $len, $mime);
}

sub kreuzberg_extract_bytes_sync_with_config ($self, $bytes, $len, $mime, $config_json) {
    return $self->{_funcs}{kreuzberg_extract_bytes_sync_with_config}->call($bytes, $len, $mime, $config_json);
}

sub kreuzberg_batch_extract_files_sync ($self, $paths_ptr, $count, $config_json) {
    return $self->{_funcs}{kreuzberg_batch_extract_files_sync}->call($paths_ptr, $count, $config_json);
}

sub kreuzberg_detect_mime_type ($self, $bytes, $len) {
    return $self->{_funcs}{kreuzberg_detect_mime_type}->call($bytes, $len);
}

sub kreuzberg_detect_mime_type_from_path ($self, $path) {
    return $self->{_funcs}{kreuzberg_detect_mime_type_from_path}->call($path);
}

sub kreuzberg_detect_mime_type_from_bytes ($self, $bytes, $len) {
    return $self->{_funcs}{kreuzberg_detect_mime_type_from_bytes}->call($bytes, $len);
}

sub kreuzberg_validate_mime_type ($self, $mime) {
    return $self->{_funcs}{kreuzberg_validate_mime_type}->call($mime);
}

sub kreuzberg_get_extensions_for_mime ($self, $mime) {
    return $self->{_funcs}{kreuzberg_get_extensions_for_mime}->call($mime);
}

sub kreuzberg_config_from_json ($self, $json) {
    return $self->{_funcs}{kreuzberg_config_from_json}->call($json);
}

sub kreuzberg_config_to_json ($self, $config) {
    return $self->{_funcs}{kreuzberg_config_to_json}->call($config);
}

sub kreuzberg_config_free ($self, $config) {
    return $self->{_funcs}{kreuzberg_config_free}->call($config);
}

sub kreuzberg_config_is_valid ($self, $config) {
    return $self->{_funcs}{kreuzberg_config_is_valid}->call($config);
}

sub kreuzberg_list_ocr_backends ($self) {
    return $self->{_funcs}{kreuzberg_list_ocr_backends}->call();
}

sub kreuzberg_list_post_processors ($self) {
    return $self->{_funcs}{kreuzberg_list_post_processors}->call();
}

sub kreuzberg_list_validators ($self) {
    return $self->{_funcs}{kreuzberg_list_validators}->call();
}

sub kreuzberg_validate_language_code ($self, $code) {
    return $self->{_funcs}{kreuzberg_validate_language_code}->call($code);
}

sub kreuzberg_validate_ocr_backend ($self, $backend) {
    return $self->{_funcs}{kreuzberg_validate_ocr_backend}->call($backend);
}

sub kreuzberg_validate_dpi ($self, $dpi) {
    return $self->{_funcs}{kreuzberg_validate_dpi}->call($dpi);
}

sub kreuzberg_validate_confidence ($self, $conf) {
    return $self->{_funcs}{kreuzberg_validate_confidence}->call($conf);
}

sub DESTROY { }

# Helper to cast opaque pointer to Perl string
sub cast_to_string ($self, $ptr) {
    return undef unless $ptr;
    return $self->{ffi}->cast('opaque', 'string', $ptr);
}

# Helper to read string from result struct at offset
sub _read_result_string ( $self, $result_ptr, $offset ) {
    return undef unless $result_ptr;

    my $ffi = $self->{ffi};

    # Read pointer at offset
    my $buffer;
    window( $buffer, $result_ptr + $offset, 8 );
    my $str_ptr = unpack( 'Q', $buffer );
    return undef unless $str_ptr;

    return $ffi->cast( 'opaque', 'string', $str_ptr );
}

sub _read_result_bool ( $self, $result_ptr, $offset ) {
    return 0 unless $result_ptr;

    # Read byte at offset
    my $buffer;
    window( $buffer, $result_ptr + $offset, 1 );
    my $byte = unpack( 'C', $buffer );
    return $byte ? 1 : 0;
}

# CExtractionResult field offsets
use constant {
    OFFSET_CONTENT                 => 0,
    OFFSET_MIME_TYPE               => 8,
    OFFSET_LANGUAGE                => 16,
    OFFSET_DATE                    => 24,
    OFFSET_SUBJECT                 => 32,
    OFFSET_TABLES_JSON             => 40,
    OFFSET_DETECTED_LANGUAGES_JSON => 48,
    OFFSET_METADATA_JSON           => 56,
    OFFSET_CHUNKS_JSON             => 64,
    OFFSET_IMAGES_JSON             => 72,
    OFFSET_PAGE_STRUCTURE_JSON     => 80,
    OFFSET_PAGES_JSON              => 88,
    OFFSET_SUCCESS                 => 96,
};

sub read_extraction_result ( $self, $result_ptr ) {
    return undef unless $result_ptr;

    my $ffi = $self->{ffi};

    # Use FFI to read the struct fields
    # The struct is 104 bytes with 12 pointers followed by a bool and padding
    # Use window to create a view into the native memory
    my $buffer;
    window( $buffer, $result_ptr, 104 );

    my @ptrs    = unpack( 'Q12', substr( $buffer, 0,  96 ) );
    my $success = unpack( 'C',   substr( $buffer, 96, 1 ) );

    my @strings;
    for my $ptr (@ptrs) {
        if ($ptr) {
            push @strings, $ffi->cast( 'opaque', 'string', $ptr );
        }
        else {
            push @strings, undef;
        }
    }

    return {
        content                 => $strings[0],
        mime_type               => $strings[1],
        language                => $strings[2],
        date                    => $strings[3],
        subject                 => $strings[4],
        tables_json             => $strings[5],
        detected_languages_json => $strings[6],
        metadata_json           => $strings[7],
        chunks_json             => $strings[8],
        images_json             => $strings[9],
        page_structure_json     => $strings[10],
        pages_json              => $strings[11],
        success                 => $success ? 1 : 0,
    };
}

sub read_batch_result ( $self, $batch_ptr, $count ) {
    return [] unless $batch_ptr && $count > 0;

    my $ffi = $self->{ffi};

    # CBatchResult layout:
    # results: *mut *mut CExtractionResult (8 bytes)
    # count: usize (8 bytes)
    # success: bool (1 byte)
    # padding: 7 bytes

    my $buffer;
    window( $buffer, $batch_ptr, 24 );
    my ( $results_ptr, $actual_count, $success ) = unpack( 'QQC', $buffer );

    my @results;
    for my $i ( 0 .. $count - 1 ) {

        # Read pointer to result at index i
        my $result_ptr_ptr = $results_ptr + ( $i * 8 );
        my $ptr_buffer;
        window( $ptr_buffer, $result_ptr_ptr, 8 );
        my $result_ptr = unpack( 'Q', $ptr_buffer );

        if ($result_ptr) {
            push @results, $self->read_extraction_result($result_ptr);
        }
        else {
            push @results, undef;
        }
    }

    return \@results;
}

1;

__END__

=head1 INTERNAL

This module is used internally and should not be called directly.
Use L<Kreuzberg> instead.

=cut
