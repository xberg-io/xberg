Import io.xberg.\*;

var config = ExtractionConfig.builder()
.pages(PageConfig.builder()
.extractPages(true)
.build())
.build();

var resultOutput = Xberg.extract(
    io.xberg.ExtractInput.builder()
        .withKind(io.xberg.ExtractInputKind.Uri)
        .withUri("document.pdf")
        .build(),
    config
);
var result = resultOutput.results().get(0);

If (result.pages() != null) {
for (var page : result.pages()) {
System.out.println("Page " + page.pageNumber() + ":");
System.out.println(" Content: " + page.content().length() + " chars");
System.out.println(" Tables: " + page.tables().size());
System.out.println(" Images: " + page.images().size());
}
}
