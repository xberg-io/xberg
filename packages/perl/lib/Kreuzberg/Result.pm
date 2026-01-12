package Kreuzberg::Result;

use strict;
use warnings;
use v5.20;
use feature 'signatures';
no warnings 'experimental::signatures';

use Carp     qw(croak);
use JSON::PP qw(decode_json);

use Kreuzberg::FFI;

=head1 NAME

Kreuzberg::Result - Document extraction result

=head1 SYNOPSIS

    use Kreuzberg;

    my $result = Kreuzberg::extract_file('/path/to/document.pdf');

    # Access extracted content
    print $result->content;
    print $result->mime_type;
    print $result->language;

    # Access metadata
    my $metadata = $result->metadata;
    my $tables = $result->tables;
    my $chunks = $result->chunks;

=head1 DESCRIPTION

This class represents the result of a document extraction operation.
It provides access to the extracted text content, metadata, and other
information about the document.

=head1 METHODS

=cut

sub new ( $class, %args ) {
    my $self = bless {
        content                 => $args{content},
        mime_type               => $args{mime_type},
        language                => $args{language},
        date                    => $args{date},
        subject                 => $args{subject},
        tables_json             => $args{tables_json},
        detected_languages_json => $args{detected_languages_json},
        metadata_json           => $args{metadata_json},
        chunks_json             => $args{chunks_json},
        images_json             => $args{images_json},
        page_structure_json     => $args{page_structure_json},
        pages_json              => $args{pages_json},
        success                 => $args{success} // 1,
    }, $class;

    return $self;
}

# Internal constructor from FFI result pointer
sub _from_ffi ( $class, $result_ptr ) {
    my $ffi  = Kreuzberg::FFI->instance;
    my $data = $ffi->read_extraction_result($result_ptr);

    return $class->new(%$data);
}

# Internal constructor from FFI batch result pointer
sub _from_batch_ffi ( $class, $batch_ptr, $count ) {
    my $ffi     = Kreuzberg::FFI->instance;
    my $results = $ffi->read_batch_result( $batch_ptr, $count );

    my @objects;
    for my $data (@$results) {
        if ($data) {
            push @objects, $class->new(%$data);
        }
        else {
            push @objects, undef;
        }
    }

    return @objects;
}

=head2 content

Returns the extracted text content of the document.

    my $text = $result->content;

=cut

sub content ($self) {
    return $self->{content};
}

=head2 mime_type

Returns the detected MIME type of the document.

    my $mime = $result->mime_type;  # e.g., 'application/pdf'

=cut

sub mime_type ($self) {
    return $self->{mime_type};
}

=head2 language

Returns the detected primary language of the document (ISO 639-1 code).

    my $lang = $result->language;  # e.g., 'en'

=cut

sub language ($self) {
    return $self->{language};
}

=head2 date

Returns the document date if available.

    my $date = $result->date;

=cut

sub date ($self) {
    return $self->{date};
}

=head2 subject

Returns the document subject if available.

    my $subject = $result->subject;

=cut

sub subject ($self) {
    return $self->{subject};
}

=head2 success

Returns true if the extraction was successful.

    if ($result->success) {
        print $result->content;
    }

=cut

sub success ($self) {
    return $self->{success} ? 1 : 0;
}

=head2 tables

Returns an array reference of extracted tables.

    my $tables = $result->tables;
    for my $table (@$tables) {
        # Process table data
    }

=cut

sub tables ($self) {
    return $self->{_tables} if exists $self->{_tables};

    if ( $self->{tables_json} ) {
        $self->{_tables} = eval { decode_json( $self->{tables_json} ) } // [];
    }
    else {
        $self->{_tables} = [];
    }

    return $self->{_tables};
}

=head2 detected_languages

Returns an array reference of detected languages with confidence scores.

    my $languages = $result->detected_languages;
    for my $lang (@$languages) {
        print "$lang->{language}: $lang->{confidence}\n";
    }

=cut

sub detected_languages ($self) {
    return $self->{_detected_languages} if exists $self->{_detected_languages};

    if ( $self->{detected_languages_json} ) {
        $self->{_detected_languages} =
          eval { decode_json( $self->{detected_languages_json} ) } // [];
    }
    else {
        $self->{_detected_languages} = [];
    }

    return $self->{_detected_languages};
}

=head2 metadata

Returns a hash reference of document metadata.

    my $metadata = $result->metadata;
    print $metadata->{author};
    print $metadata->{title};

=cut

sub metadata ($self) {
    return $self->{_metadata} if exists $self->{_metadata};

    if ( $self->{metadata_json} ) {
        $self->{_metadata} =
          eval { decode_json( $self->{metadata_json} ) } // {};
    }
    else {
        $self->{_metadata} = {};
    }

    return $self->{_metadata};
}

=head2 chunks

Returns an array reference of text chunks.

    my $chunks = $result->chunks;
    for my $chunk (@$chunks) {
        print $chunk->{text};
    }

=cut

sub chunks ($self) {
    return $self->{_chunks} if exists $self->{_chunks};

    if ( $self->{chunks_json} ) {
        $self->{_chunks} = eval { decode_json( $self->{chunks_json} ) } // [];
    }
    else {
        $self->{_chunks} = [];
    }

    return $self->{_chunks};
}

=head2 images

Returns an array reference of extracted images.

    my $images = $result->images;
    for my $image (@$images) {
        # Process image data
    }

=cut

sub images ($self) {
    return $self->{_images} if exists $self->{_images};

    if ( $self->{images_json} ) {
        $self->{_images} = eval { decode_json( $self->{images_json} ) } // [];
    }
    else {
        $self->{_images} = [];
    }

    return $self->{_images};
}

=head2 page_structure

Returns the page structure information.

    my $structure = $result->page_structure;

=cut

sub page_structure ($self) {
    return $self->{_page_structure} if exists $self->{_page_structure};

    if ( $self->{page_structure_json} ) {
        $self->{_page_structure} =
          eval { decode_json( $self->{page_structure_json} ) } // {};
    }
    else {
        $self->{_page_structure} = {};
    }

    return $self->{_page_structure};
}

=head2 pages

Returns an array reference of per-page content.

    my $pages = $result->pages;
    for my $page (@$pages) {
        print "Page $page->{number}: $page->{content}\n";
    }

=cut

sub pages ($self) {
    return $self->{_pages} if exists $self->{_pages};

    if ( $self->{pages_json} ) {
        $self->{_pages} = eval { decode_json( $self->{pages_json} ) } // [];
    }
    else {
        $self->{_pages} = [];
    }

    return $self->{_pages};
}

=head2 to_hash

Returns the result as a hash reference.

    my $hash = $result->to_hash;

=cut

sub to_hash ($self) {
    return {
        content            => $self->content,
        mime_type          => $self->mime_type,
        language           => $self->language,
        date               => $self->date,
        subject            => $self->subject,
        tables             => $self->tables,
        detected_languages => $self->detected_languages,
        metadata           => $self->metadata,
        chunks             => $self->chunks,
        images             => $self->images,
        page_structure     => $self->page_structure,
        pages              => $self->pages,
        success            => $self->success,
    };
}

1;

__END__

=head1 SEE ALSO

L<Kreuzberg>

=cut
