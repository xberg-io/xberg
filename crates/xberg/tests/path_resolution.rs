//! Integration tests for image path resolution in markup extractors.

use std::path::PathBuf;
use xberg::ExtractionConfig;
use xberg::ImageExtractionConfig;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/path_resolution/fixtures")
}

fn config_with_images() -> ExtractionConfig {
    ExtractionConfig {
        images: Some(ImageExtractionConfig {
            extract_images: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[tokio::test]
async fn test_markdown_resolves_images() {
    let path = fixtures_dir().join("markdown_with_images.md");
    let config = config_with_images();
    let result = xberg::extract_file(&path, None, &config).await.unwrap();

    let images = result.images.as_ref().expect("should have images");
    // Should resolve the 2 local images but NOT the https:// URL
    assert_eq!(images.len(), 2, "expected 2 resolved images, got {}", images.len());

    // Verify image data is non-empty
    for img in images {
        assert!(!img.data.is_empty(), "image data should not be empty");
        assert_eq!(img.format, "png");
    }
}

#[tokio::test]
async fn test_markdown_bytes_no_resolution() {
    let path = fixtures_dir().join("markdown_with_images.md");
    let content = std::fs::read(&path).unwrap();
    let config = config_with_images();
    let result = xberg::extract_bytes(&content, "text/markdown", &config).await.unwrap();

    // extract_bytes has no file path context, so no image resolution should happen
    let image_count = result.images.as_ref().map_or(0, |imgs| imgs.len());
    assert_eq!(image_count, 0, "extract_bytes should not resolve local images");
}

#[cfg(feature = "office")]
#[tokio::test]
async fn test_latex_resolves_images() {
    let path = fixtures_dir().join("latex_with_images.tex");
    let config = config_with_images();
    let result = xberg::extract_file(&path, None, &config).await.unwrap();

    let images = result.images.as_ref().expect("should have images");
    assert_eq!(images.len(), 2, "expected 2 resolved images, got {}", images.len());
}

#[cfg(feature = "office")]
#[tokio::test]
async fn test_rst_resolves_images() {
    let path = fixtures_dir().join("rst_with_images.rst");
    let config = config_with_images();
    let result = xberg::extract_file(&path, Some("text/x-rst"), &config).await.unwrap();

    let images = result.images.as_ref().expect("should have images");
    assert_eq!(images.len(), 2, "expected 2 resolved images, got {}", images.len());
}

#[cfg(feature = "office")]
#[tokio::test]
async fn test_orgmode_resolves_images() {
    let path = fixtures_dir().join("orgmode_with_images.org");
    let config = config_with_images();
    let result = xberg::extract_file(&path, Some("text/x-org"), &config).await.unwrap();

    let images = result.images.as_ref().expect("should have images");
    assert_eq!(images.len(), 2, "expected 2 resolved images, got {}", images.len());
}

#[cfg(feature = "office")]
#[tokio::test]
async fn test_typst_resolves_images() {
    let path = fixtures_dir().join("typst_with_images.typ");
    let config = config_with_images();
    let result = xberg::extract_file(&path, Some("application/x-typst"), &config)
        .await
        .unwrap();

    let images = result.images.as_ref().expect("should have images");
    assert_eq!(images.len(), 2, "expected 2 resolved images, got {}", images.len());
}

#[tokio::test]
async fn test_djot_resolves_images() {
    let path = fixtures_dir().join("djot_with_images.djot");
    let config = config_with_images();
    let result = xberg::extract_file(&path, Some("text/djot"), &config).await.unwrap();

    let images = result.images.as_ref().expect("should have images");
    assert_eq!(images.len(), 2, "expected 2 resolved images, got {}", images.len());
}

#[tokio::test]
async fn test_traversal_blocked() {
    // Create a temp markdown file that references a traversal path
    let tmp_dir = std::env::temp_dir().join("xberg_path_test");
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let md_path = tmp_dir.join("traversal.md");
    std::fs::write(&md_path, "![evil](../../../etc/passwd)\n![ok](images/test_image.png)\n").unwrap();

    let config = config_with_images();
    let result = xberg::extract_file(&md_path, Some("text/markdown"), &config)
        .await
        .unwrap();

    // Neither should resolve: traversal is blocked, and images/ doesn't exist in tmp
    let image_count = result.images.as_ref().map_or(0, |imgs| imgs.len());
    assert_eq!(image_count, 0, "traversal paths should not resolve to images");

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp_dir);
}
