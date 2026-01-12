package Kreuzberg::Config;

use strict;
use warnings;
use v5.20;
use feature 'signatures';
no warnings 'experimental::signatures';

use Carp     qw(croak);
use JSON::PP qw(encode_json);

=head1 NAME

Kreuzberg::Config - Configuration for document extraction

=head1 SYNOPSIS

    use Kreuzberg::Config;

    my $config = Kreuzberg::Config->new(
        enable_ocr      => 1,
        ocr_language    => 'eng',
        extract_tables  => 1,
        extract_images  => 0,
    );

    my $result = Kreuzberg::extract_file('/path/to/file.pdf', $config);

=head1 DESCRIPTION

This class represents the configuration options for document extraction.

=head1 CONSTRUCTOR

=head2 new(%options)

Creates a new configuration object.

    my $config = Kreuzberg::Config->new(
        enable_ocr => 1,
        ocr_language => 'eng',
    );

=head3 Options

=over 4

=item * enable_ocr - Enable OCR for image-based documents (default: 0)

=item * ocr_language - OCR language code (default: 'eng')

=item * ocr_backend - OCR backend to use ('tesseract', 'easyocr', 'paddleocr')

=item * extract_tables - Extract tables from documents (default: 0)

=item * extract_images - Extract images from documents (default: 0)

=item * enable_chunking - Enable text chunking (default: 0)

=item * chunk_size - Maximum chunk size in characters (default: 1000)

=item * chunk_overlap - Overlap between chunks (default: 200)

=item * detect_language - Enable language detection (default: 0)

=item * pdf_dpi - DPI for PDF rendering (default: 300)

=item * pdf_extract_images - Extract images from PDFs (default: 0)

=item * output_format - Output format ('text', 'markdown', 'html')

=item * token_reduction - Token reduction level ('none', 'light', 'medium', 'aggressive')

=back

=cut

sub new ( $class, %args ) {
    my $self = bless {

        # OCR settings
        enable_ocr     => $args{enable_ocr} // 0,
        ocr_language   => $args{ocr_language},
        ocr_backend    => $args{ocr_backend},
        ocr_dpi        => $args{ocr_dpi},
        ocr_confidence => $args{ocr_confidence},

        # Extraction settings
        extract_tables   => $args{extract_tables}   // 0,
        extract_images   => $args{extract_images}   // 0,
        extract_metadata => $args{extract_metadata} // 1,

        # Chunking settings
        enable_chunking => $args{enable_chunking} // 0,
        chunk_size      => $args{chunk_size},
        chunk_overlap   => $args{chunk_overlap},

        # Language detection
        detect_language => $args{detect_language} // 0,

        # PDF settings
        pdf_dpi            => $args{pdf_dpi},
        pdf_extract_images => $args{pdf_extract_images},
        pdf_password       => $args{pdf_password},

        # Output settings
        output_format   => $args{output_format},
        token_reduction => $args{token_reduction},

        # Page settings
        page_numbers => $args{page_numbers},

        # Keyword extraction
        extract_keywords  => $args{extract_keywords} // 0,
        keyword_algorithm => $args{keyword_algorithm},
        max_keywords      => $args{max_keywords},
    }, $class;

    return $self;
}

=head1 METHODS

=head2 to_json

Converts the configuration to a JSON string for FFI.

    my $json = $config->to_json;

=cut

sub to_json ($self) {
    my %config;

    # OCR config
    if ( $self->{enable_ocr} || $self->{ocr_language} || $self->{ocr_backend} )
    {
        $config{ocr} = {};
        $config{ocr}{enabled} = $self->{enable_ocr} ? \1 : \0
          if defined $self->{enable_ocr};
        $config{ocr}{backend} = $self->{ocr_backend}
          if defined $self->{ocr_backend};
        $config{ocr}{language} = $self->{ocr_language}
          if defined $self->{ocr_language};
        $config{ocr}{dpi} = $self->{ocr_dpi}
          if defined $self->{ocr_dpi};
    }

    # Table extraction
    if ( $self->{extract_tables} ) {
        $config{tables} = { enabled => \1 };
    }

    # Image extraction
    if ( $self->{extract_images} ) {
        $config{images} = { extract_images => \1 };
    }

    # Chunking config
    if ( $self->{enable_chunking} || $self->{chunk_size} ) {
        $config{chunking} = {};
        $config{chunking}{enabled} = $self->{enable_chunking} ? \1 : \0
          if defined $self->{enable_chunking};
        $config{chunking}{max_characters} = $self->{chunk_size}
          if defined $self->{chunk_size};
        $config{chunking}{overlap} = $self->{chunk_overlap}
          if defined $self->{chunk_overlap};
    }

    # Language detection
    if ( $self->{detect_language} ) {
        $config{language_detection} = { enabled => \1 };
    }

    # PDF config
    if ( $self->{pdf_dpi} || $self->{pdf_extract_images} || $self->{pdf_password} ) {
        $config{pdf} = {};
        $config{pdf}{dpi} = $self->{pdf_dpi}
          if defined $self->{pdf_dpi};
        $config{pdf}{extract_images} = $self->{pdf_extract_images} ? \1 : \0
          if defined $self->{pdf_extract_images};
        $config{pdf}{password} = $self->{pdf_password}
          if defined $self->{pdf_password};
    }

    # Token reduction
    if ( defined $self->{token_reduction} ) {
        $config{token_reduction} = { mode => $self->{token_reduction} };
    }

    # Page numbers
    if ( defined $self->{page_numbers} ) {
        $config{pages} = { page_numbers => $self->{page_numbers} };
    }

    # Keyword extraction
    if ( $self->{extract_keywords} || $self->{keyword_algorithm} ) {
        $config{keywords} = {};
        $config{keywords}{enabled} = $self->{extract_keywords} ? \1 : \0
          if defined $self->{extract_keywords};
        $config{keywords}{algorithm} = $self->{keyword_algorithm}
          if defined $self->{keyword_algorithm};
        $config{keywords}{max_keywords} = $self->{max_keywords}
          if defined $self->{max_keywords};
    }

    return encode_json( \%config );
}

=head2 Accessors

The following accessor methods are available:

=over 4

=item * enable_ocr

=item * ocr_language

=item * ocr_backend

=item * extract_tables

=item * extract_images

=item * enable_chunking

=item * chunk_size

=item * chunk_overlap

=item * detect_language

=item * pdf_dpi

=item * output_format

=item * token_reduction

=back

=cut

# Generate accessors
for my $attr (
    qw(
    enable_ocr ocr_language ocr_backend ocr_dpi ocr_confidence
    extract_tables extract_images extract_metadata
    enable_chunking chunk_size chunk_overlap
    detect_language
    pdf_dpi pdf_extract_images pdf_password
    output_format token_reduction
    page_numbers
    extract_keywords keyword_algorithm max_keywords
    )
  )
{
    no strict 'refs';
    *{$attr} = sub ( $self, $value = undef ) {
        if ( defined $value ) {
            $self->{$attr} = $value;
            return $self;
        }
        return $self->{$attr};
    };
}

1;

__END__

=head1 SEE ALSO

L<Kreuzberg>

=cut
