Import { extractSync } from '@xberg-io/xberg';

Const result = extractSync('document.pdf');

If (result.metadata.page_structure?.boundaries) {
const encoder = new TextEncoder();
const contentBytes = encoder.encode(result.content);

For (const boundary of result.metadata.page_structure.boundaries.slice(0, 3)) {
const pageBytes = contentBytes.slice(boundary.byteStart, boundary.byteEnd);
const pageText = new TextDecoder().decode(pageBytes);

    console.log(`Page ${boundary.pageNumber}:`);
    console.log(`  Byte range: ${boundary.byteStart}-${boundary.byteEnd}`);
    console.log(`  Preview: ${pageText.substring(0, 100)}...`);

}
}
