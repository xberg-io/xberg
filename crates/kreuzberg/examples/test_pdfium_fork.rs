use kreuzberg::{ExtractionConfig, extract_file};
use std::time::Instant;

#[tokio::main]
async fn main() {
    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };

    println!("Testing PDF extraction with cleaned pdfium-render fork...\n");

    println!("Test 1: fake_memo.pdf");
    let start = Instant::now();
    match extract_file("test_documents/pdfs/fake_memo.pdf", None, &config).await {
        Ok(result) => {
            let duration = start.elapsed();
            println!("  ✓ Success! Duration: {:?}", duration);
            println!("  ✓ Text length: {} chars", result.content.len());
        }
        Err(e) => {
            println!("  ✗ Failed: {}", e);
            std::process::exit(1);
        }
    }

    println!("\nTest 2: Warm iteration");
    let start = Instant::now();
    match extract_file("test_documents/pdfs/fake_memo.pdf", None, &config).await {
        Ok(result) => {
            let duration = start.elapsed();
            println!("  ✓ Success! Duration: {:?}", duration);
            println!("  ✓ Text length: {} chars", result.content.len());
        }
        Err(e) => {
            println!("  ✗ Failed: {}", e);
            std::process::exit(1);
        }
    }

    println!("\nTest 3: Academic Paper (18 fonts)");
    let start = Instant::now();
    match extract_file(
        "test_documents/pdfs/a_comprehensive_study_of_convergent_and_commutative_replicated_data_types.pdf",
        None,
        &config,
    )
    .await
    {
        Ok(result) => {
            let duration = start.elapsed();
            println!("  ✓ Success! Duration: {:?}", duration);
            println!("  ✓ Text length: {} chars", result.content.len());
        }
        Err(e) => {
            println!("  ✗ Failed: {}", e);
            std::process::exit(1);
        }
    }

    println!("\n✅ All tests passed! Cleaned pdfium-render fork is working correctly.");
}
