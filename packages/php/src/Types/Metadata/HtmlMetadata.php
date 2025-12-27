<?php

declare(strict_types=1);

namespace Kreuzberg\Types\Metadata;

/**
 * HTML metadata extracted from web documents.
 *
 * Contains metadata extracted from HTML documents including Open Graph data,
 * Twitter Card information, headers, links, images, and structured data.
 */
class HtmlMetadata
{
    /** @var string[] */
    public array $keywords = [];

    public ?string $canonicalUrl = null;

    /** @var array<string, string> */
    public array $openGraph = [];

    /** @var array<string, string> */
    public array $twitterCard = [];

    public ?string $language = null;

    public ?string $textDirection = null;

    /** @var array<string, string> */
    public array $metaTags = [];

    /** @var HeaderMetadata[] */
    public array $headers = [];

    /** @var LinkMetadata[] */
    public array $links = [];

    /** @var ImageMetadata[] */
    public array $images = [];

    /** @var StructuredData[] */
    public array $structuredData = [];

    /**
     * @param string[] $keywords
     * @param array<string, string> $openGraph
     * @param array<string, string> $twitterCard
     * @param array<string, string> $metaTags
     * @param HeaderMetadata[] $headers
     * @param LinkMetadata[] $links
     * @param ImageMetadata[] $images
     * @param StructuredData[] $structuredData
     */
    public function __construct(
        array $keywords = [],
        ?string $canonicalUrl = null,
        array $openGraph = [],
        array $twitterCard = [],
        ?string $language = null,
        ?string $textDirection = null,
        array $metaTags = [],
        array $headers = [],
        array $links = [],
        array $images = [],
        array $structuredData = [],
    ) {
        $this->keywords = $keywords;
        $this->canonicalUrl = $canonicalUrl;
        $this->openGraph = $openGraph;
        $this->twitterCard = $twitterCard;
        $this->language = $language;
        $this->textDirection = $textDirection;
        $this->metaTags = $metaTags;
        $this->headers = $headers;
        $this->links = $links;
        $this->images = $images;
        $this->structuredData = $structuredData;
    }

    /**
     * Create HtmlMetadata from array returned by extension.
     *
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        /** @var string[] */
        $keywords = $data['keywords'] ?? [];
        if (!is_array($keywords)) {
            $keywords = [];
        }

        /** @var string|null */
        $canonicalUrl = $data['canonical_url'] ?? null;

        /** @var array<string, string> */
        $openGraph = $data['open_graph'] ?? [];
        if (!is_array($openGraph)) {
            $openGraph = [];
        }

        /** @var array<string, string> */
        $twitterCard = $data['twitter_card'] ?? [];
        if (!is_array($twitterCard)) {
            $twitterCard = [];
        }

        /** @var string|null */
        $language = $data['language'] ?? null;

        /** @var string|null */
        $textDirection = $data['text_direction'] ?? null;

        /** @var array<string, string> */
        $metaTags = $data['meta_tags'] ?? [];
        if (!is_array($metaTags)) {
            $metaTags = [];
        }

        /** @var HeaderMetadata[] */
        $headers = [];
        if (isset($data['headers']) && is_array($data['headers'])) {
            foreach ($data['headers'] as $headerData) {
                if (is_array($headerData)) {
                    $headers[] = new HeaderMetadata(
                        level: (int) ($headerData['level'] ?? 1),
                        text: (string) ($headerData['text'] ?? ''),
                        depth: (int) ($headerData['depth'] ?? 0),
                        htmlOffset: (int) ($headerData['html_offset'] ?? 0),
                        id: $headerData['id'] ?? null,
                    );
                }
            }
        }

        /** @var LinkMetadata[] */
        $links = [];
        if (isset($data['links']) && is_array($data['links'])) {
            foreach ($data['links'] as $linkData) {
                if (is_array($linkData)) {
                    $links[] = new LinkMetadata(
                        href: (string) ($linkData['href'] ?? ''),
                        text: (string) ($linkData['text'] ?? ''),
                        linkType: (string) ($linkData['link_type'] ?? 'other'),
                        title: $linkData['title'] ?? null,
                        rel: is_array($linkData['rel'] ?? null) ? $linkData['rel'] : [],
                        attributes: is_array($linkData['attributes'] ?? null) ? $linkData['attributes'] : [],
                    );
                }
            }
        }

        /** @var ImageMetadata[] */
        $images = [];
        if (isset($data['images']) && is_array($data['images'])) {
            foreach ($data['images'] as $imageData) {
                if (is_array($imageData)) {
                    $images[] = new ImageMetadata(
                        src: (string) ($imageData['src'] ?? ''),
                        imageType: (string) ($imageData['image_type'] ?? 'external'),
                        alt: $imageData['alt'] ?? null,
                        title: $imageData['title'] ?? null,
                        dimensions: is_array($imageData['dimensions'] ?? null) ? $imageData['dimensions'] : null,
                        attributes: is_array($imageData['attributes'] ?? null) ? $imageData['attributes'] : [],
                    );
                }
            }
        }

        /** @var StructuredData[] */
        $structuredData = [];
        if (isset($data['structured_data']) && is_array($data['structured_data'])) {
            foreach ($data['structured_data'] as $sdData) {
                if (is_array($sdData)) {
                    $structuredData[] = new StructuredData(
                        dataType: (string) ($sdData['data_type'] ?? ''),
                        rawJson: (string) ($sdData['raw_json'] ?? ''),
                        schemaType: $sdData['schema_type'] ?? null,
                    );
                }
            }
        }

        return new self(
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
    }
}
