```rust title="simple_benchmark.rs"
use std::time::Instant;
use xberg::{extract, ExtractInput, ExtractionConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };
    let file_path = "document.pdf";
    let input = ExtractInput::from_uri(file_path);
    let num_runs = 10;

    let start = Instant::now();
    for _ in 0..num_runs {
        let _ = extract(input.clone(), &config).await?;
    }
    let sequential_duration = start.elapsed().as_secs_f64();
    let avg_sequential = sequential_duration / num_runs as f64;

    println!("Sequential extraction ({} runs):", num_runs);
    println!("  - Total time: {:.3}s", sequential_duration);
    println!("  - Average: {:.3}s per extraction", avg_sequential);

    let start = Instant::now();
    let mut tasks = vec![];
    for _ in 0..num_runs {
        tasks.push(extract(input.clone(), &config));
    }
    let results = futures::future::join_all(tasks).await;
    for result in results {
        result?;
    }
    let async_duration = start.elapsed().as_secs_f64();

    println!("\nAsync extraction ({} parallel runs):", num_runs);
    println!("  - Total time: {:.3}s", async_duration);
    println!("  - Average: {:.3}s per extraction", async_duration / num_runs as f64);
    println!("  - Speedup: {:.1}x", sequential_duration / async_duration);

    let config_cached = ExtractionConfig {
        use_cache: true,
        ..Default::default()
    };

    println!("\nFirst extraction (populates cache)...");
    let start = Instant::now();
    let _result1 = extract(input.clone(), &config_cached).await?;
    let first_duration = start.elapsed().as_secs_f64();
    println!("  - Time: {:.3}s", first_duration);

    println!("Second extraction (from cache)...");
    let start = Instant::now();
    let _result2 = extract(input, &config_cached).await?;
    let cached_duration = start.elapsed().as_secs_f64();
    println!("  - Time: {:.3}s", cached_duration);
    println!("  - Cache speedup: {:.1}x", first_duration / cached_duration);

    Ok(())
}
```
