# Latest Benchmark Results

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
