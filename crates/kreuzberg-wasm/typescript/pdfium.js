/**
 * Mock PDFium module for WASM tests
 * This provides a fallback for browser environments where PDFium is optional
 */

export default async function initPdfium() {
	// Return a mock PDFium module
	return {
		// Dummy implementation for testing
	};
}
