use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'Kreuzberg',
    AUTHOR           => [
        q{Na'aman Hirschfeld <nhirschfeld@gmail.com>},
        q{Jason Kiniry <jason.kiniry@gmail.com>},
    ],
    VERSION_FROM     => 'lib/Kreuzberg.pm',
    ABSTRACT_FROM    => 'lib/Kreuzberg.pm',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.020',

    CONFIGURE_REQUIRES => {
        'ExtUtils::MakeMaker' => '0',
    },

    BUILD_REQUIRES => {
        'Test::More'      => '0',
        'Test::Exception' => '0',
    },

    PREREQ_PM => {
        'FFI::Platypus'       => '2.00',
        'FFI::Platypus::Memory' => '0',
        'JSON::PP'            => '0',
        'Carp'                => '0',
        'File::Spec'          => '0',
    },

    TEST_REQUIRES => {
        'Test::More'      => '0.98',
        'Test::Exception' => '0',
        'File::Temp'      => '0',
    },

    META_MERGE => {
        'meta-spec' => { version => 2 },
        resources => {
            repository => {
                type => 'git',
                url  => 'https://github.com/kreuzberg-dev/kreuzberg.git',
                web  => 'https://github.com/kreuzberg-dev/kreuzberg',
            },
            bugtracker => {
                web => 'https://github.com/kreuzberg-dev/kreuzberg/issues',
            },
            homepage => 'https://kreuzberg.dev',
        },
        keywords => [
            'document-intelligence',
            'document-extraction',
            'ocr',
            'pdf',
            'text-extraction',
            'rust',
            'ffi',
        ],
        x_authority => 'cpan:KREUZBERG',
    },

    dist  => { COMPRESS => 'gzip -9f', SUFFIX => 'gz', },
    clean => { FILES => 'Kreuzberg-*' },
);

# Post-install message
sub MY::postamble {
    return <<'MAKE';
install ::
	$(NOECHO) $(ECHO) ""
	$(NOECHO) $(ECHO) "============================================"
	$(NOECHO) $(ECHO) "Kreuzberg Perl bindings installed!"
	$(NOECHO) $(ECHO) ""
	$(NOECHO) $(ECHO) "NOTE: You must have the Kreuzberg FFI library"
	$(NOECHO) $(ECHO) "installed on your system. Get it from:"
	$(NOECHO) $(ECHO) "  https://github.com/kreuzberg-dev/kreuzberg"
	$(NOECHO) $(ECHO) ""
	$(NOECHO) $(ECHO) "Set KREUZBERG_LIB_PATH if the library is in"
	$(NOECHO) $(ECHO) "a non-standard location."
	$(NOECHO) $(ECHO) "============================================"
	$(NOECHO) $(ECHO) ""
MAKE
}
