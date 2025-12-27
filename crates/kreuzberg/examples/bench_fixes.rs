use kreuzberg::{ExtractionConfig, extract_file_sync};
use std::path::PathBuf;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let test_pdfs = [
        (
            "a_comprehensive_study_of_convergent_and_commutative_replicated_data_types.pdf",
            "Academic Paper (18 fonts)",
        ),
        (
            "5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf",
            "Intel PDF (5 fonts)",
        ),
        ("fake_memo.pdf", "Tiny Memo (3-5 fonts)"),
    ];

    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };

    println!("=== PDFium Fork Fixes Benchmark ===\n");
    println!("Testing warm execution fix and font overhead fix\n");

    for (file, description) in &test_pdfs {
        let path = PathBuf::from(format!("test_documents/pdfs/{}", file));
        println!("=== {} ===", description);
        println!("File: {}\n", file);

        let start = Instant::now();
        let result = extract_file_sync(&path, None, &config)?;
        let cold = start.elapsed();
        println!("Cold start:  {:>8.2} ms", cold.as_secs_f64() * 1000.0);
        println!("Text length: {} chars\n", result.content.len());

        let mut warm_times = Vec::new();
        for i in 1..=5 {
            let start = Instant::now();
            let _ = extract_file_sync(&path, None, &config)?;
            let warm = start.elapsed();
            warm_times.push(warm);
            let speedup = cold.as_micros() as f64 / warm.as_micros() as f64;
            println!(
                "Warm {:>2}:     {:>8.2} ms ({:>5.2}x faster than cold)",
                i,
                warm.as_secs_f64() * 1000.0,
                speedup
            );
        }

        let avg_warm = warm_times.iter().sum::<std::time::Duration>() / warm_times.len() as u32;
        let avg_speedup = cold.as_micros() as f64 / avg_warm.as_micros() as f64;
        println!(
            "\nAverage warm: {:>8.2} ms ({:>5.2}x faster than cold)",
            avg_warm.as_secs_f64() * 1000.0,
            avg_speedup
        );
        println!("\n{}\n", "=".repeat(60));
    }

    println!("\n=== Success Criteria ===");
    println!("✓ Warm Execution Fix:");
    println!("  - Warm times should be 1-3x faster than cold (realistic)");
    println!("  - NOT 100-700x faster (the bug we fixed)");
    println!("\n✓ Font Overhead Fix:");
    println!("  - Academic Paper cold: ~130-145ms (matches baseline)");
    println!("  - NOT 180-195ms (the regression we fixed)");

    Ok(())
}
