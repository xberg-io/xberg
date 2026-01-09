# Latest Benchmark Results

!!! note "Page Moved"
    This page has moved to [/benchmarks/](index.md). This page is kept for backwards compatibility and shows the same content.

<div class="benchmark-dashboard" markdown="1">

## Interactive Dashboard

The charts below are generated from the most recent benchmark workflow run.

!!! info "Data Source"
    Results from the latest successful [benchmark workflow run](https://github.com/kreuzberg-dev/kreuzberg/actions/workflows/benchmarks.yaml).

    The benchmark date is displayed in the visualization header. Visualizations are automatically updated when new benchmarks complete.

</div>

<div class="full-width" markdown="1">

<!-- Embedded React Visualizer -->
<iframe src="/benchmarks/app/"
        width="100%"
        height="2000px"
        frameborder="0"
        sandbox="allow-same-origin allow-scripts allow-popups allow-forms"
        style="border: 1px solid var(--md-default-fg-color--lightest); border-radius: 4px;"
        title="Interactive Benchmark Visualizer">
  <p>Your browser does not support iframes. Please visit the <a href="/benchmarks/app/">interactive benchmark visualizer</a> directly.</p>
</iframe>

<script>
(function() {
  const iframe = document.querySelector('iframe[title="Interactive Benchmark Visualizer"]');
  if (!iframe) return;

  const sendTheme = () => {
    const scheme = document.body.getAttribute('data-md-color-scheme');
    const isDark = scheme === 'slate';
    iframe.contentWindow.postMessage({
      type: 'theme',
      value: isDark ? 'dark' : 'light'
    }, '*');
  };

  // Send theme when iframe loads
  iframe.addEventListener('load', () => setTimeout(sendTheme, 100));

  // Send theme if iframe already loaded
  if (iframe.contentDocument && iframe.contentDocument.readyState === 'complete') {
    setTimeout(sendTheme, 100);
  }

  // Watch for theme changes
  const observer = new MutationObserver(sendTheme);
  observer.observe(document.body, {
    attributes: true,
    attributeFilter: ['data-md-color-scheme']
  });
})();
</script>

</div>

## Direct Data Access

For programmatic access or custom analysis:

- **Aggregated Data**: [/benchmarks/data/aggregated.json](../data/aggregated.json)
- **Metadata**: [/benchmarks/data/metadata.json](../data/metadata.json)
- **Workflow Runs**: [View on GitHub Actions](https://github.com/kreuzberg-dev/kreuzberg/actions/workflows/benchmarks.yaml)

!!! tip "API Access"
    The aggregated benchmark data is available as JSON for programmatic access and custom analysis.

## Run Your Own

See [Methodology](methodology.md) for instructions to run benchmarks locally with your own documents.
