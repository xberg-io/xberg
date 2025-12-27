<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Unit;

use Kreuzberg\Kreuzberg;
use Kreuzberg\Types\Metadata\HeaderMetadata;
use Kreuzberg\Types\Metadata\HtmlMetadata;
use Kreuzberg\Types\Metadata\ImageMetadata;
use Kreuzberg\Types\Metadata\LinkMetadata;
use Kreuzberg\Types\Metadata\StructuredData;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Comprehensive tests for PHP metadata types in the Kreuzberg library.
 *
 * Verifies the structure, creation, deserialization, and integration of:
 * - HtmlMetadata: Container for all HTML-related metadata
 * - HeaderMetadata: HTML header/heading information
 * - LinkMetadata: Hyperlink information
 * - ImageMetadata: Image metadata with dimensions and attributes
 * - StructuredData: Schema.org and other structured data
 */
#[CoversClass(HtmlMetadata::class)]
#[CoversClass(HeaderMetadata::class)]
#[CoversClass(LinkMetadata::class)]
#[CoversClass(ImageMetadata::class)]
#[CoversClass(StructuredData::class)]
final class MetadataTypesTest extends TestCase
{
    #[Test]
    public function testHtmlMetadataStructure(): void
    {
        $metadata = new HtmlMetadata();

        $this->assertIsArray($metadata->keywords);
        $this->assertNull($metadata->canonicalUrl);
        $this->assertIsArray($metadata->openGraph);
        $this->assertIsArray($metadata->twitterCard);
        $this->assertNull($metadata->language);
        $this->assertNull($metadata->textDirection);
        $this->assertIsArray($metadata->metaTags);
        $this->assertIsArray($metadata->headers);
        $this->assertIsArray($metadata->links);
        $this->assertIsArray($metadata->images);
        $this->assertIsArray($metadata->structuredData);

        $this->assertEmpty($metadata->keywords);
        $this->assertEmpty($metadata->openGraph);
        $this->assertEmpty($metadata->twitterCard);
        $this->assertEmpty($metadata->metaTags);
        $this->assertEmpty($metadata->headers);
        $this->assertEmpty($metadata->links);
        $this->assertEmpty($metadata->images);
        $this->assertEmpty($metadata->structuredData);
    }

    #[Test]
    public function testKeywordsIsArray(): void
    {
        $keywords = ['seo', 'metadata', 'html'];
        $metadata = new HtmlMetadata(keywords: $keywords);

        $this->assertIsArray($metadata->keywords);
        $this->assertCount(3, $metadata->keywords);
        $this->assertSame('seo', $metadata->keywords[0]);
        $this->assertSame('metadata', $metadata->keywords[1]);
        $this->assertSame('html', $metadata->keywords[2]);
    }

    #[Test]
    public function testCanonicalUrlRenamed(): void
    {
        $canonicalUrl = 'https://example.com/page';
        $metadata = new HtmlMetadata(canonicalUrl: $canonicalUrl);

        $this->assertTrue(property_exists($metadata, 'canonicalUrl'));
        $this->assertSame($canonicalUrl, $metadata->canonicalUrl);
    }

    #[Test]
    public function testOpenGraphIsArray(): void
    {
        $openGraph = [
            'og:title' => 'Page Title',
            'og:description' => 'Page Description',
            'og:image' => 'https://example.com/image.jpg',
        ];
        $metadata = new HtmlMetadata(openGraph: $openGraph);

        $this->assertIsArray($metadata->openGraph);
        $this->assertCount(3, $metadata->openGraph);
        $this->assertSame('Page Title', $metadata->openGraph['og:title']);
        $this->assertSame('Page Description', $metadata->openGraph['og:description']);
        $this->assertSame('https://example.com/image.jpg', $metadata->openGraph['og:image']);
    }

    #[Test]
    public function testTwitterCardIsArray(): void
    {
        $twitterCard = [
            'twitter:card' => 'summary_large_image',
            'twitter:title' => 'Tweet Title',
            'twitter:image' => 'https://example.com/twitter.jpg',
        ];
        $metadata = new HtmlMetadata(twitterCard: $twitterCard);

        $this->assertIsArray($metadata->twitterCard);
        $this->assertCount(3, $metadata->twitterCard);
        $this->assertSame('summary_large_image', $metadata->twitterCard['twitter:card']);
        $this->assertSame('Tweet Title', $metadata->twitterCard['twitter:title']);
    }


    #[Test]
    public function testHeaderMetadataCreation(): void
    {
        $header = new HeaderMetadata(
            level: 1,
            text: 'Main Title',
            depth: 0,
            htmlOffset: 42,
            id: 'main-title',
        );

        $this->assertSame(1, $header->level);
        $this->assertSame('Main Title', $header->text);
        $this->assertSame(0, $header->depth);
        $this->assertSame(42, $header->htmlOffset);
        $this->assertSame('main-title', $header->id);
    }

    #[Test]
    public function testHeaderMetadataWithoutId(): void
    {
        $header = new HeaderMetadata(
            level: 2,
            text: 'Subheading',
            depth: 1,
            htmlOffset: 100,
        );

        $this->assertSame(2, $header->level);
        $this->assertSame('Subheading', $header->text);
        $this->assertNull($header->id);
    }

    #[Test]
    public function testLinkMetadataCreation(): void
    {
        $link = new LinkMetadata(
            href: 'https://example.com',
            text: 'Example Link',
            linkType: 'external',
            title: 'Visit Example',
            rel: ['noopener', 'noreferrer'],
            attributes: ['data-tracking' => 'link-1'],
        );

        $this->assertSame('https://example.com', $link->href);
        $this->assertSame('Example Link', $link->text);
        $this->assertSame('external', $link->linkType);
        $this->assertSame('Visit Example', $link->title);
        $this->assertIsArray($link->rel);
        $this->assertCount(2, $link->rel);
        $this->assertContains('noopener', $link->rel);
        $this->assertContains('noreferrer', $link->rel);
        $this->assertIsArray($link->attributes);
        $this->assertSame('link-1', $link->attributes['data-tracking']);
    }

    #[Test]
    public function testLinkMetadataDefaults(): void
    {
        $link = new LinkMetadata(
            href: '/about',
            text: 'About',
        );

        $this->assertSame('/about', $link->href);
        $this->assertSame('About', $link->text);
        $this->assertSame('other', $link->linkType);
        $this->assertNull($link->title);
        $this->assertEmpty($link->rel);
        $this->assertEmpty($link->attributes);
    }

    #[Test]
    public function testImageMetadataCreation(): void
    {
        $image = new ImageMetadata(
            src: 'https://example.com/image.jpg',
            imageType: 'external',
            alt: 'Example Image',
            title: 'Example',
            dimensions: [800, 600],
            attributes: ['loading' => 'lazy'],
        );

        $this->assertSame('https://example.com/image.jpg', $image->src);
        $this->assertSame('external', $image->imageType);
        $this->assertSame('Example Image', $image->alt);
        $this->assertSame('Example', $image->title);
        $this->assertIsArray($image->dimensions);
        $this->assertSame([800, 600], $image->dimensions);
        $this->assertSame('lazy', $image->attributes['loading']);
    }

    #[Test]
    public function testImageMetadataWithoutDimensions(): void
    {
        $image = new ImageMetadata(
            src: '/images/logo.png',
        );

        $this->assertSame('/images/logo.png', $image->src);
        $this->assertSame('external', $image->imageType);
        $this->assertNull($image->alt);
        $this->assertNull($image->title);
        $this->assertNull($image->dimensions);
        $this->assertEmpty($image->attributes);
    }

    #[Test]
    public function testStructuredDataCreation(): void
    {
        $structuredData = new StructuredData(
            dataType: 'application/ld+json',
            rawJson: '{"@context":"https://schema.org","@type":"Article"}',
            schemaType: 'Article',
        );

        $this->assertSame('application/ld+json', $structuredData->dataType);
        $this->assertSame('{"@context":"https://schema.org","@type":"Article"}', $structuredData->rawJson);
        $this->assertSame('Article', $structuredData->schemaType);
    }

    #[Test]
    public function testStructuredDataWithoutSchemaType(): void
    {
        $structuredData = new StructuredData(
            dataType: 'application/ld+json',
            rawJson: '{}',
        );

        $this->assertSame('application/ld+json', $structuredData->dataType);
        $this->assertSame('{}', $structuredData->rawJson);
        $this->assertNull($structuredData->schemaType);
    }


    #[Test]
    public function testHtmlMetadataFromArray(): void
    {
        $data = [
            'keywords' => ['php', 'metadata', 'extraction'],
            'canonical_url' => 'https://example.com/article',
            'open_graph' => [
                'og:title' => 'Article Title',
                'og:type' => 'article',
            ],
            'twitter_card' => [
                'twitter:card' => 'summary',
            ],
            'language' => 'en',
            'text_direction' => 'ltr',
            'meta_tags' => [
                'viewport' => 'width=device-width, initial-scale=1',
            ],
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertSame(['php', 'metadata', 'extraction'], $metadata->keywords);
        $this->assertSame('https://example.com/article', $metadata->canonicalUrl);
        $this->assertCount(2, $metadata->openGraph);
        $this->assertSame('Article Title', $metadata->openGraph['og:title']);
        $this->assertCount(1, $metadata->twitterCard);
        $this->assertSame('en', $metadata->language);
        $this->assertSame('ltr', $metadata->textDirection);
        $this->assertCount(1, $metadata->metaTags);
    }

    #[Test]
    public function testHeaderMetadataDeserialization(): void
    {
        $data = [
            'headers' => [
                [
                    'level' => 1,
                    'text' => 'Title',
                    'depth' => 0,
                    'html_offset' => 10,
                    'id' => 'title',
                ],
                [
                    'level' => 2,
                    'text' => 'Section',
                    'depth' => 1,
                    'html_offset' => 50,
                ],
            ],
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertCount(2, $metadata->headers);
        $this->assertInstanceOf(HeaderMetadata::class, $metadata->headers[0]);
        $this->assertInstanceOf(HeaderMetadata::class, $metadata->headers[1]);

        $this->assertSame(1, $metadata->headers[0]->level);
        $this->assertSame('Title', $metadata->headers[0]->text);
        $this->assertSame('title', $metadata->headers[0]->id);

        $this->assertSame(2, $metadata->headers[1]->level);
        $this->assertSame('Section', $metadata->headers[1]->text);
        $this->assertNull($metadata->headers[1]->id);
    }

    #[Test]
    public function testLinkMetadataDeserialization(): void
    {
        $data = [
            'links' => [
                [
                    'href' => 'https://example.com',
                    'text' => 'Home',
                    'link_type' => 'navigation',
                    'title' => 'Go Home',
                    'rel' => ['home'],
                    'attributes' => ['data-nav' => 'true'],
                ],
                [
                    'href' => '/about',
                    'text' => 'About Us',
                    'link_type' => 'internal',
                ],
            ],
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertCount(2, $metadata->links);
        $this->assertInstanceOf(LinkMetadata::class, $metadata->links[0]);
        $this->assertInstanceOf(LinkMetadata::class, $metadata->links[1]);

        $this->assertSame('https://example.com', $metadata->links[0]->href);
        $this->assertSame('Home', $metadata->links[0]->text);
        $this->assertSame('navigation', $metadata->links[0]->linkType);
        $this->assertContains('home', $metadata->links[0]->rel);
        $this->assertSame('true', $metadata->links[0]->attributes['data-nav']);

        $this->assertSame('/about', $metadata->links[1]->href);
        $this->assertEmpty($metadata->links[1]->rel);
    }

    #[Test]
    public function testImageMetadataDeserialization(): void
    {
        $data = [
            'images' => [
                [
                    'src' => 'https://example.com/image1.jpg',
                    'image_type' => 'external',
                    'alt' => 'First Image',
                    'title' => 'Image 1',
                    'dimensions' => [1920, 1080],
                    'attributes' => ['data-lightbox' => 'gallery'],
                ],
                [
                    'src' => '/images/logo.png',
                    'image_type' => 'local',
                ],
            ],
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertCount(2, $metadata->images);
        $this->assertInstanceOf(ImageMetadata::class, $metadata->images[0]);
        $this->assertInstanceOf(ImageMetadata::class, $metadata->images[1]);

        $this->assertSame('https://example.com/image1.jpg', $metadata->images[0]->src);
        $this->assertSame('external', $metadata->images[0]->imageType);
        $this->assertSame('First Image', $metadata->images[0]->alt);
        $this->assertSame([1920, 1080], $metadata->images[0]->dimensions);
        $this->assertSame('gallery', $metadata->images[0]->attributes['data-lightbox']);

        $this->assertSame('/images/logo.png', $metadata->images[1]->src);
        $this->assertNull($metadata->images[1]->alt);
        $this->assertNull($metadata->images[1]->dimensions);
    }

    #[Test]
    public function testStructuredDataDeserialization(): void
    {
        $data = [
            'structured_data' => [
                [
                    'data_type' => 'application/ld+json',
                    'raw_json' => '{"@context":"https://schema.org","@type":"Article","headline":"Test"}',
                    'schema_type' => 'Article',
                ],
                [
                    'data_type' => 'microdata',
                    'raw_json' => '{"type":"Person"}',
                ],
            ],
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertCount(2, $metadata->structuredData);
        $this->assertInstanceOf(StructuredData::class, $metadata->structuredData[0]);
        $this->assertInstanceOf(StructuredData::class, $metadata->structuredData[1]);

        $this->assertSame('application/ld+json', $metadata->structuredData[0]->dataType);
        $this->assertStringContainsString('Article', $metadata->structuredData[0]->rawJson);
        $this->assertSame('Article', $metadata->structuredData[0]->schemaType);

        $this->assertSame('microdata', $metadata->structuredData[1]->dataType);
        $this->assertNull($metadata->structuredData[1]->schemaType);
    }


    #[Test]
    public function testCompleteHtmlMetadataWithAllFields(): void
    {
        $data = [
            'keywords' => ['web', 'development', 'php'],
            'canonical_url' => 'https://example.com/post/123',
            'open_graph' => [
                'og:title' => 'Web Development Guide',
                'og:description' => 'A comprehensive guide to web development',
                'og:image' => 'https://example.com/og-image.jpg',
                'og:url' => 'https://example.com/post/123',
            ],
            'twitter_card' => [
                'twitter:card' => 'summary_large_image',
                'twitter:title' => 'Web Development Guide',
                'twitter:description' => 'A comprehensive guide',
                'twitter:image' => 'https://example.com/twitter-image.jpg',
            ],
            'language' => 'en',
            'text_direction' => 'ltr',
            'meta_tags' => [
                'viewport' => 'width=device-width, initial-scale=1.0',
                'charset' => 'utf-8',
            ],
            'headers' => [
                ['level' => 1, 'text' => 'Web Development', 'depth' => 0, 'html_offset' => 0, 'id' => 'web-dev'],
                ['level' => 2, 'text' => 'Getting Started', 'depth' => 1, 'html_offset' => 50, 'id' => 'getting-started'],
            ],
            'links' => [
                [
                    'href' => 'https://example.com/docs',
                    'text' => 'Documentation',
                    'link_type' => 'external',
                    'rel' => ['external'],
                ],
            ],
            'images' => [
                [
                    'src' => 'https://example.com/hero.jpg',
                    'image_type' => 'external',
                    'alt' => 'Hero Image',
                    'dimensions' => [1200, 600],
                ],
            ],
            'structured_data' => [
                [
                    'data_type' => 'application/ld+json',
                    'raw_json' => '{"@type":"Article"}',
                    'schema_type' => 'Article',
                ],
            ],
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertCount(3, $metadata->keywords);
        $this->assertSame('https://example.com/post/123', $metadata->canonicalUrl);
        $this->assertCount(4, $metadata->openGraph);
        $this->assertCount(4, $metadata->twitterCard);
        $this->assertSame('en', $metadata->language);
        $this->assertSame('ltr', $metadata->textDirection);
        $this->assertCount(2, $metadata->metaTags);
        $this->assertCount(2, $metadata->headers);
        $this->assertCount(1, $metadata->links);
        $this->assertCount(1, $metadata->images);
        $this->assertCount(1, $metadata->structuredData);
    }

    #[Test]
    public function testMetadataArrayWithOnlyRequiredFields(): void
    {
        $data = [
            'keywords' => [],
            'open_graph' => [],
            'twitter_card' => [],
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertEmpty($metadata->keywords);
        $this->assertNull($metadata->canonicalUrl);
        $this->assertEmpty($metadata->openGraph);
        $this->assertEmpty($metadata->twitterCard);
        $this->assertNull($metadata->language);
        $this->assertEmpty($metadata->headers);
        $this->assertEmpty($metadata->links);
        $this->assertEmpty($metadata->images);
        $this->assertEmpty($metadata->structuredData);
    }

    #[Test]
    public function testMetadataArrayWithMissingFields(): void
    {
        $data = [];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertEmpty($metadata->keywords);
        $this->assertNull($metadata->canonicalUrl);
        $this->assertEmpty($metadata->openGraph);
        $this->assertEmpty($metadata->twitterCard);
        $this->assertNull($metadata->language);
        $this->assertNull($metadata->textDirection);
        $this->assertEmpty($metadata->metaTags);
        $this->assertEmpty($metadata->headers);
        $this->assertEmpty($metadata->links);
        $this->assertEmpty($metadata->images);
        $this->assertEmpty($metadata->structuredData);
    }


    #[Test]
    public function testMetadataEmptyHtml(): void
    {
        $data = [
            'keywords' => [],
            'open_graph' => [],
            'twitter_card' => [],
            'meta_tags' => [],
            'headers' => [],
            'links' => [],
            'images' => [],
            'structured_data' => [],
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertEmpty($metadata->keywords);
        $this->assertEmpty($metadata->openGraph);
        $this->assertEmpty($metadata->twitterCard);
        $this->assertEmpty($metadata->metaTags);
        $this->assertEmpty($metadata->headers);
        $this->assertEmpty($metadata->links);
        $this->assertEmpty($metadata->images);
        $this->assertEmpty($metadata->structuredData);
    }

    #[Test]
    public function testMetadataNullOptionalFields(): void
    {
        $data = [
            'canonical_url' => null,
            'language' => null,
            'text_direction' => null,
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertNull($metadata->canonicalUrl);
        $this->assertNull($metadata->language);
        $this->assertNull($metadata->textDirection);
    }

    #[Test]
    public function testMetadataEmptyCollections(): void
    {
        $metadata = new HtmlMetadata(
            keywords: [],
            openGraph: [],
            twitterCard: [],
            metaTags: [],
            headers: [],
            links: [],
            images: [],
            structuredData: [],
        );

        $this->assertIsArray($metadata->keywords);
        $this->assertEmpty($metadata->keywords);
        $this->assertIsArray($metadata->openGraph);
        $this->assertEmpty($metadata->openGraph);
        $this->assertIsArray($metadata->twitterCard);
        $this->assertEmpty($metadata->twitterCard);
        $this->assertIsArray($metadata->metaTags);
        $this->assertEmpty($metadata->metaTags);
        $this->assertIsArray($metadata->headers);
        $this->assertEmpty($metadata->headers);
        $this->assertIsArray($metadata->links);
        $this->assertEmpty($metadata->links);
        $this->assertIsArray($metadata->images);
        $this->assertEmpty($metadata->images);
        $this->assertIsArray($metadata->structuredData);
        $this->assertEmpty($metadata->structuredData);
    }

    #[Test]
    public function testInvalidTypeHandlingInDeserialization(): void
    {
        $data = [
            'keywords' => 'not-an-array',
            'open_graph' => 'string-value',
            'twitter_card' => 123,
            'meta_tags' => false,
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertIsArray($metadata->keywords);
        $this->assertEmpty($metadata->keywords);
        $this->assertIsArray($metadata->openGraph);
        $this->assertEmpty($metadata->openGraph);
        $this->assertIsArray($metadata->twitterCard);
        $this->assertEmpty($metadata->twitterCard);
        $this->assertIsArray($metadata->metaTags);
        $this->assertEmpty($metadata->metaTags);
    }

    #[Test]
    public function testNestedMetadataInvalidTypeHandling(): void
    {
        $data = [
            'headers' => 'not-an-array',
            'links' => 123,
            'images' => false,
            'structured_data' => null,
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertIsArray($metadata->headers);
        $this->assertEmpty($metadata->headers);
        $this->assertIsArray($metadata->links);
        $this->assertEmpty($metadata->links);
        $this->assertIsArray($metadata->images);
        $this->assertEmpty($metadata->images);
        $this->assertIsArray($metadata->structuredData);
        $this->assertEmpty($metadata->structuredData);
    }

    #[Test]
    public function testHeaderMetadataWithVaryingLevels(): void
    {
        $headers = [
            new HeaderMetadata(level: 1, text: 'H1', depth: 0, htmlOffset: 0),
            new HeaderMetadata(level: 2, text: 'H2', depth: 1, htmlOffset: 10),
            new HeaderMetadata(level: 3, text: 'H3', depth: 2, htmlOffset: 20),
            new HeaderMetadata(level: 4, text: 'H4', depth: 3, htmlOffset: 30),
            new HeaderMetadata(level: 5, text: 'H5', depth: 4, htmlOffset: 40),
            new HeaderMetadata(level: 6, text: 'H6', depth: 5, htmlOffset: 50),
        ];

        $metadata = new HtmlMetadata(headers: $headers);

        $this->assertCount(6, $metadata->headers);
        foreach (range(1, 6) as $i) {
            $this->assertSame($i, $metadata->headers[$i - 1]->level);
        }
    }

    #[Test]
    public function testLinkMetadataWithMultipleRelValues(): void
    {
        $link = new LinkMetadata(
            href: 'https://example.com',
            text: 'External Link',
            rel: ['external', 'noopener', 'noreferrer', 'nofollow'],
        );

        $this->assertCount(4, $link->rel);
        $this->assertContains('external', $link->rel);
        $this->assertContains('noopener', $link->rel);
        $this->assertContains('noreferrer', $link->rel);
        $this->assertContains('nofollow', $link->rel);
    }

    #[Test]
    public function testImageMetadataVariousTypes(): void
    {
        $imageTypes = ['external', 'local', 'embedded', 'data-uri'];

        foreach ($imageTypes as $type) {
            $image = new ImageMetadata(
                src: 'image.jpg',
                imageType: $type,
            );
            $this->assertSame($type, $image->imageType);
        }
    }

    #[Test]
    public function testStructuredDataVariousFormats(): void
    {
        $jsonLd = new StructuredData(
            dataType: 'application/ld+json',
            rawJson: '{"@context":"https://schema.org"}',
            schemaType: 'BlogPosting',
        );

        $microdata = new StructuredData(
            dataType: 'microdata',
            rawJson: '<div itemscope itemtype="https://schema.org/Person"></div>',
        );

        $microformat = new StructuredData(
            dataType: 'microformat',
            rawJson: '<a class="h-entry"><p class="p-name">Example</p></a>',
        );

        $this->assertSame('application/ld+json', $jsonLd->dataType);
        $this->assertSame('microdata', $microdata->dataType);
        $this->assertSame('microformat', $microformat->dataType);

        $this->assertSame('BlogPosting', $jsonLd->schemaType);
        $this->assertNull($microdata->schemaType);
        $this->assertNull($microformat->schemaType);
    }

    #[Test]
    public function testMultipleHeadersWithSameLevel(): void
    {
        $data = [
            'headers' => [
                ['level' => 2, 'text' => 'First Section', 'depth' => 1, 'html_offset' => 0],
                ['level' => 2, 'text' => 'Second Section', 'depth' => 1, 'html_offset' => 100],
                ['level' => 2, 'text' => 'Third Section', 'depth' => 1, 'html_offset' => 200],
            ],
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertCount(3, $metadata->headers);
        $this->assertSame('First Section', $metadata->headers[0]->text);
        $this->assertSame('Second Section', $metadata->headers[1]->text);
        $this->assertSame('Third Section', $metadata->headers[2]->text);

        foreach ($metadata->headers as $header) {
            $this->assertSame(2, $header->level);
            $this->assertSame(1, $header->depth);
        }
    }

    #[Test]
    public function testOpenGraphMinimalSet(): void
    {
        $data = [
            'open_graph' => [
                'og:title' => 'Title',
                'og:type' => 'website',
            ],
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertCount(2, $metadata->openGraph);
        $this->assertArrayHasKey('og:title', $metadata->openGraph);
        $this->assertArrayHasKey('og:type', $metadata->openGraph);
    }

    #[Test]
    public function testTwitterCardMinimalSet(): void
    {
        $data = [
            'twitter_card' => [
                'twitter:card' => 'summary',
            ],
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertCount(1, $metadata->twitterCard);
        $this->assertArrayHasKey('twitter:card', $metadata->twitterCard);
        $this->assertSame('summary', $metadata->twitterCard['twitter:card']);
    }

    #[Test]
    public function testConstructorPreservesAllValues(): void
    {
        $keywords = ['a', 'b', 'c'];
        $canonicalUrl = 'https://example.com';
        $openGraph = ['og:title' => 'Title'];
        $twitterCard = ['twitter:card' => 'summary'];
        $language = 'en';
        $textDirection = 'ltr';
        $metaTags = ['viewport' => 'width=device-width'];
        $headers = [new HeaderMetadata(1, 'Title', 0, 0)];
        $links = [new LinkMetadata('/', 'Home')];
        $images = [new ImageMetadata('/img.jpg')];
        $structuredData = [new StructuredData('application/ld+json', '{}')];

        $metadata = new HtmlMetadata(
            keywords: $keywords,
            canonicalUrl: $canonicalUrl,
            openGraph: $openGraph,
            twitterCard: $twitterCard,
            language: $language,
            textDirection: $textDirection,
            metaTags: $metaTags,
            headers: $headers,
            links: $links,
            images: $images,
            structuredData: $structuredData,
        );

        $this->assertSame($keywords, $metadata->keywords);
        $this->assertSame($canonicalUrl, $metadata->canonicalUrl);
        $this->assertSame($openGraph, $metadata->openGraph);
        $this->assertSame($twitterCard, $metadata->twitterCard);
        $this->assertSame($language, $metadata->language);
        $this->assertSame($textDirection, $metadata->textDirection);
        $this->assertSame($metaTags, $metadata->metaTags);
        $this->assertSame($headers, $metadata->headers);
        $this->assertSame($links, $metadata->links);
        $this->assertSame($images, $metadata->images);
        $this->assertSame($structuredData, $metadata->structuredData);
    }

    #[Test]
    public function testStringConversionForMissingIntFields(): void
    {
        $data = [
            'headers' => [
                [
                    'level' => '2',
                    'text' => 'Section',
                    'depth' => '1',
                    'html_offset' => '50',
                ],
            ],
        ];

        $metadata = HtmlMetadata::fromArray($data);

        $this->assertIsInt($metadata->headers[0]->level);
        $this->assertIsInt($metadata->headers[0]->depth);
        $this->assertIsInt($metadata->headers[0]->htmlOffset);
        $this->assertSame(2, $metadata->headers[0]->level);
        $this->assertSame(1, $metadata->headers[0]->depth);
        $this->assertSame(50, $metadata->headers[0]->htmlOffset);
    }

    #[Test]
    public function testArrayValuesPreservedInAttributes(): void
    {
        $attributes = [
            'data-config' => 'value1',
            'data-options' => 'value2',
            'class' => 'image-class',
            'id' => 'unique-id',
        ];

        $link = new LinkMetadata('/', 'Test', attributes: $attributes);

        $this->assertCount(4, $link->attributes);
        foreach ($attributes as $key => $value) {
            $this->assertArrayHasKey($key, $link->attributes);
            $this->assertSame($value, $link->attributes[$key]);
        }
    }


    #[Test]
    #[RequiresPhpExtension('kreuzberg')]
    public function testExtractHtmlReturnsMetadataObject(): void
    {
        if (!extension_loaded('kreuzberg')) {
            $this->markTestSkipped('Kreuzberg extension is not loaded');
        }

        $htmlContent = <<<'HTML'
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>Test Page</title>
                <meta name="keywords" content="testing,metadata,extraction">
                <meta name="description" content="A test page for metadata extraction">
                <link rel="canonical" href="https://example.com/test-page">
                <meta property="og:title" content="Test Page OG">
                <meta property="og:type" content="article">
                <meta property="og:image" content="https://example.com/image.jpg">
            </head>
            <body>
                <h1>Main Heading</h1>
                <h2>Subheading</h2>
                <p>Some content here.</p>
                <a href="https://example.com">External Link</a>
                <a href="/about">Internal Link</a>
                <img src="https://example.com/photo.jpg" alt="Test Image">
            </body>
            </html>
            HTML;

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractBytes($htmlContent, 'text/html');

        $this->assertIsString($result->content);

        if ($result->metadata->hasCustom('html_metadata')) {
            $htmlMetadataArray = $result->metadata->getCustom('html_metadata');
            $this->assertIsArray($htmlMetadataArray);

            $htmlMetadata = HtmlMetadata::fromArray($htmlMetadataArray);
            $this->assertInstanceOf(HtmlMetadata::class, $htmlMetadata);

            $this->assertIsArray($htmlMetadata->keywords);
            $this->assertIsArray($htmlMetadata->openGraph);
            $this->assertIsArray($htmlMetadata->headers);
            $this->assertIsArray($htmlMetadata->links);
            $this->assertIsArray($htmlMetadata->images);
        }
    }

    #[Test]
    #[RequiresPhpExtension('kreuzberg')]
    public function testExtractComplexHtmlAllFields(): void
    {
        if (!extension_loaded('kreuzberg')) {
            $this->markTestSkipped('Kreuzberg extension is not loaded');
        }

        $htmlContent = <<<'HTML'
            <!DOCTYPE html>
            <html lang="en" dir="ltr">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <meta http-equiv="X-UA-Compatible" content="ie=edge">

                <title>Complete Article Test</title>
                <meta name="description" content="A comprehensive test article with all metadata fields">
                <meta name="keywords" content="article,testing,kreuzberg,php,metadata,extraction">
                <meta name="author" content="Test Author">
                <link rel="canonical" href="https://example.com/complete-article">

                <!-- Open Graph -->
                <meta property="og:title" content="Complete Article Test">
                <meta property="og:type" content="article">
                <meta property="og:description" content="A comprehensive test article">
                <meta property="og:url" content="https://example.com/complete-article">
                <meta property="og:image" content="https://example.com/og-image.jpg">
                <meta property="og:site_name" content="Test Site">

                <!-- Twitter Card -->
                <meta name="twitter:card" content="summary_large_image">
                <meta name="twitter:title" content="Complete Article Test">
                <meta name="twitter:description" content="A comprehensive test article">
                <meta name="twitter:image" content="https://example.com/twitter-image.jpg">
                <meta name="twitter:site" content="@testsite">

                <!-- Structured Data -->
                <script type="application/ld+json">
                {
                    "@context": "https://schema.org",
                    "@type": "Article",
                    "headline": "Complete Article Test",
                    "image": "https://example.com/article-image.jpg",
                    "datePublished": "2024-01-01T08:00:00+00:00",
                    "dateModified": "2024-01-15T09:00:00+00:00",
                    "author": {
                        "@type": "Person",
                        "name": "Test Author"
                    }
                }
                </script>
            </head>
            <body>
                <header>
                    <h1 id="main-title">Complete Article Test</h1>
                    <p>Published by Test Author</p>
                </header>

                <article>
                    <h2 id="introduction">Introduction</h2>
                    <p>This is the introduction section with important content.</p>

                    <h3 id="section-1">Section 1: Overview</h3>
                    <p>Detailed overview of the topic.</p>

                    <h3 id="section-2">Section 2: Details</h3>
                    <p>More detailed information here.</p>

                    <nav>
                        <ul>
                            <li><a href="/" title="Home Page">Home</a></li>
                            <li><a href="/about" title="About Us">About</a></li>
                            <li><a href="https://external.com" rel="noopener noreferrer">External Site</a></li>
                        </ul>
                    </nav>

                    <figure>
                        <img src="https://example.com/main-image.jpg" alt="Main Article Image" loading="lazy">
                        <figcaption>Figure 1: Main article illustration</figcaption>
                    </figure>

                    <figure>
                        <img src="/images/local-image.png" alt="Local Image" title="Local Illustration">
                        <figcaption>Figure 2: Local image</figcaption>
                    </figure>
                </article>

                <footer>
                    <p>&copy; 2024 Test Site. All rights reserved.</p>
                </footer>
            </body>
            </html>
            HTML;

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractBytes($htmlContent, 'text/html');

        $this->assertIsString($result->content);
        $this->assertNotEmpty($result->content);

        if ($result->metadata->hasCustom('html_metadata')) {
            $htmlMetadataArray = $result->metadata->getCustom('html_metadata');
            $htmlMetadata = HtmlMetadata::fromArray($htmlMetadataArray);

            $this->assertIsArray($htmlMetadata->keywords);
            $this->assertNotEmpty($htmlMetadata->keywords);

            $this->assertIsNotNull($htmlMetadata->canonicalUrl);
            $this->assertStringContainsString('example.com', $htmlMetadata->canonicalUrl);

            $this->assertIsArray($htmlMetadata->openGraph);
            $this->assertNotEmpty($htmlMetadata->openGraph);
            $this->assertArrayHasKey('og:title', $htmlMetadata->openGraph);
            $this->assertArrayHasKey('og:type', $htmlMetadata->openGraph);

            $this->assertIsArray($htmlMetadata->twitterCard);
            $this->assertNotEmpty($htmlMetadata->twitterCard);
            $this->assertArrayHasKey('twitter:card', $htmlMetadata->twitterCard);

            $this->assertSame('en', $htmlMetadata->language);
            $this->assertSame('ltr', $htmlMetadata->textDirection);

            $this->assertIsArray($htmlMetadata->metaTags);

            $this->assertIsArray($htmlMetadata->headers);
            $this->assertNotEmpty($htmlMetadata->headers);
            $this->assertInstanceOf(HeaderMetadata::class, $htmlMetadata->headers[0]);

            $this->assertIsArray($htmlMetadata->links);
            $this->assertNotEmpty($htmlMetadata->links);
            foreach ($htmlMetadata->links as $link) {
                $this->assertInstanceOf(LinkMetadata::class, $link);
            }

            $this->assertIsArray($htmlMetadata->images);
            $this->assertNotEmpty($htmlMetadata->images);
            foreach ($htmlMetadata->images as $image) {
                $this->assertInstanceOf(ImageMetadata::class, $image);
            }

            $this->assertIsArray($htmlMetadata->structuredData);
            $this->assertNotEmpty($htmlMetadata->structuredData);
            foreach ($htmlMetadata->structuredData as $sd) {
                $this->assertInstanceOf(StructuredData::class, $sd);
            }
        }
    }

    #[Test]
    #[RequiresPhpExtension('kreuzberg')]
    public function testExtractInvalidHtmlHandlesGracefully(): void
    {
        if (!extension_loaded('kreuzberg')) {
            $this->markTestSkipped('Kreuzberg extension is not loaded');
        }

        $kreuzberg = new Kreuzberg();

        $emptyHtml = '';
        $result = $kreuzberg->extractBytes($emptyHtml, 'text/html');
        $this->assertIsString($result->content);

        $malformedHtml = <<<'HTML'
            <!DOCTYPE html>
            <html>
            <head>
                <title>Malformed Page
                <meta name="description" content="Missing closing tags
            </head>
            <body>
                <h1>Incomplete heading
                <p>Paragraph without closing tag
            </body>
            HTML;

        $result = $kreuzberg->extractBytes($malformedHtml, 'text/html');
        $this->assertIsString($result->content);

        if ($result->metadata->hasCustom('html_metadata')) {
            $htmlMetadataArray = $result->metadata->getCustom('html_metadata');
            $htmlMetadata = HtmlMetadata::fromArray($htmlMetadataArray);
            $this->assertInstanceOf(HtmlMetadata::class, $htmlMetadata);
        }

        $largeHtml = '<!DOCTYPE html><html><head><title>Large Doc</title></head><body>';
        for ($i = 0; $i < 1000; $i++) {
            $largeHtml .= '<p>Paragraph ' . $i . '</p>';
        }
        for ($i = 0; $i < 100; $i++) {
            $largeHtml .= '<a href="https://example.com/' . $i . '">Link ' . $i . '</a>';
        }
        $largeHtml .= '</body></html>';

        $result = $kreuzberg->extractBytes($largeHtml, 'text/html');
        $this->assertIsString($result->content);

        if ($result->metadata->hasCustom('html_metadata')) {
            $htmlMetadataArray = $result->metadata->getCustom('html_metadata');
            $htmlMetadata = HtmlMetadata::fromArray($htmlMetadataArray);
            $this->assertInstanceOf(HtmlMetadata::class, $htmlMetadata);
        }
    }

    #[Test]
    #[RequiresPhpExtension('kreuzberg')]
    public function testExtractSpecialCharactersInMetadata(): void
    {
        if (!extension_loaded('kreuzberg')) {
            $this->markTestSkipped('Kreuzberg extension is not loaded');
        }

        $htmlContent = <<<'HTML'
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <title>Test with Special Characters: 你好, مرحبا, здравствуй</title>
                <meta name="keywords" content="emoji 😀🎉, accents: café, naïve, résumé">
                <meta name="description" content="Description with quotes: &quot;test&quot; and apostrophes: it's working">
                <meta property="og:title" content="Special Chars: &lt;tag&gt; &amp; entities">
                <meta property="og:description" content="Unicode: ™®©℠ and symbols: ±×÷">
            </head>
            <body>
                <h1>测试标题 - Test Title - ٹیسٹ ٹائٹل</h1>
                <h2>Café au Lait</h2>
                <h3>Naïve approach to "testing"</h3>
                <p>Content with special chars: &lt;script&gt;, &quot;quotes&quot;, &apos;apostrophe&apos;</p>
                <a href="/page-with-é">Link with special char</a>
                <a href="/emoji-😀">Link with emoji</a>
                <img src="image.jpg" alt="Image of 北京 (Beijing) with café sign">
                <img src="photo.png" alt="Photo: It's amazing™">
            </body>
            </html>
            HTML;

        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractBytes($htmlContent, 'text/html');

        $this->assertIsString($result->content);

        if ($result->metadata->hasCustom('html_metadata')) {
            $htmlMetadataArray = $result->metadata->getCustom('html_metadata');
            $htmlMetadata = HtmlMetadata::fromArray($htmlMetadataArray);

            $this->assertIsArray($htmlMetadata->keywords);

            $this->assertIsArray($htmlMetadata->openGraph);

            $this->assertIsArray($htmlMetadata->headers);

            $this->assertIsArray($htmlMetadata->links);

            $this->assertIsArray($htmlMetadata->images);
            foreach ($htmlMetadata->images as $image) {
                $this->assertInstanceOf(ImageMetadata::class, $image);
                if ($image->alt) {
                    $this->assertIsString($image->alt);
                }
            }
        }
    }

    #[Test]
    #[RequiresPhpExtension('kreuzberg')]
    public function testExtractLargeHtmlPerformance(): void
    {
        if (!extension_loaded('kreuzberg')) {
            $this->markTestSkipped('Kreuzberg extension is not loaded');
        }

        $htmlContent = <<<'HTML'
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <title>Large Document Performance Test</title>
                <meta name="description" content="Performance test with large HTML">
                <meta property="og:title" content="Large Document">
            </head>
            <body>
            HTML;

        for ($i = 1; $i <= 500; $i++) {
            $level = ($i % 6) + 1;
            $htmlContent .= '<h' . $level . '>Header ' . $i . '</h' . $level . '>';
        }

        for ($i = 1; $i <= 2000; $i++) {
            $htmlContent .= '<p>Paragraph ' . $i . ' with <a href="/link-' . $i . '">link</a> and ';
            $htmlContent .= 'some <strong>bold</strong> content.</p>';
        }

        for ($i = 1; $i <= 500; $i++) {
            $htmlContent .= '<img src="image-' . $i . '.jpg" alt="Image ' . $i . '" loading="lazy">';
        }

        $htmlContent .= '</body></html>';

        $startTime = microtime(true);
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractBytes($htmlContent, 'text/html');
        $endTime = microtime(true);

        $this->assertIsString($result->content);
        $this->assertNotEmpty($result->content);

        $duration = $endTime - $startTime;
        $this->assertLessThan(30, $duration, 'Large HTML extraction took too long: ' . $duration . 's');

        if ($result->metadata->hasCustom('html_metadata')) {
            $htmlMetadataArray = $result->metadata->getCustom('html_metadata');
            $htmlMetadata = HtmlMetadata::fromArray($htmlMetadataArray);

            $this->assertIsArray($htmlMetadata->headers);
            $this->assertGreaterThan(0, count($htmlMetadata->headers));

            $this->assertIsArray($htmlMetadata->links);
            $this->assertGreaterThan(0, count($htmlMetadata->links));

            $this->assertIsArray($htmlMetadata->images);
            $this->assertGreaterThan(0, count($htmlMetadata->images));
        }
    }
}
