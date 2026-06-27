Import { extractSync } from '@xberg-io/xberg';

Const result = extractSync('document.pdf', null, { pages: { extractPages: true } });

If (result.pages) {
for (const page of result.pages) {
console.log(`Page ${page.pageNumber}:`);
console.log(`  Content: ${page.content.length} chars`);
console.log(`  Tables: ${page.tables.length}`);
console.log(`  Images: ${page.images.length}`);
}
}
