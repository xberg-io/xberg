Import io.xberg.\*;

var config = ExtractionConfig.builder()
.pages(PageConfig.builder()
.extractPages(true)
.build())
.build();

var result = Xberg.extractSync("document.pdf", config);

If (result.pages() != null) {
for (var page : result.pages()) {
System.out.println("Page " + page.pageNumber() + ":");
System.out.println(" Content: " + page.content().length() + " chars");
System.out.println(" Tables: " + page.tables().size());
System.out.println(" Images: " + page.images().size());
}
}
