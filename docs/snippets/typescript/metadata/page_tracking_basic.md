Import { extractFileSync } from '@kreuzberg/node';

Const result = extractFileSync('document.pdf', null, { pages: { extractPages: true } });

If (result.pages) {
for (const page of result.pages) {
console.log(`Page ${page.pageNumber}:`);
console.log(`  Content: ${page.content.length} chars`);
console.log(`  Tables: ${page.tables.length}`);
console.log(`  Images: ${page.images.length}`);
}
}
