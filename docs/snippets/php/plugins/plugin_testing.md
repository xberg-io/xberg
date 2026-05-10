```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use PHPUnit\Framework\TestCase;

class CustomPluginTest extends TestCase {
    private object $plugin;
    private object $mockResult;
    private object $mockConfig;

    protected function setUp(): void {
        // Create mock extraction result
        $this->mockResult = (object)[
            'content' => 'Test content with some words',
            'mime_type' => 'text/plain',
            'metadata' => [],
            'tables' => [],
            'detected_languages' => ['eng'],
            'chunks' => null,
            'images' => null,
        ];

        // Create mock extraction config
        $this->mockConfig = (object)[];

        // Initialize plugin
        $this->plugin = new WordCountProcessor();
        $this->plugin->initialize();
    }

    protected function tearDown(): void {
        $this->plugin->shutdown();
    }

    public function testPluginInitialization(): void {
        $this->assertNotNull($this->plugin);
        $this->assertEqual($this->plugin->name(), "word-count");
    }

    public function testPluginProcessing(): void {
        // Test that plugin processes results
        $this->plugin->process($this->mockResult, $this->mockConfig);
        
        $this->assertArrayHasKey('word_count', $this->mockResult->metadata);
        $this->assertGreaterThan(0, $this->mockResult->metadata['word_count']);
    }

    public function testShouldProcess(): void {
        // Test shouldProcess logic
        $this->assertTrue($this->plugin->shouldProcess($this->mockResult, $this->mockConfig));
        
        // Empty content should not process
        $emptyResult = (object)['content' => ''];
        $this->assertFalse($this->plugin->shouldProcess($emptyResult, $this->mockConfig));
    }

    public function testProcessingStage(): void {
        $stage = $this->plugin->processingStage();
        $this->assertEqual($stage, "Early");
    }

    public function testPriority(): void {
        $priority = $this->plugin->priority();
        $this->assertGreaterThanOrEqual(0, $priority);
        $this->assertLessThanOrEqual(255, $priority);
    }
}
```
