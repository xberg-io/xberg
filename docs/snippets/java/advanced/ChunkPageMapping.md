Import io.xberg.\*;

var config = ExtractionConfig.builder()
.chunking(ChunkingConfig.builder()
.chunkSize(500)
.overlap(50)
.build())
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

If (result.chunks() != null) {
for (var chunk : result.chunks()) {
if (chunk.metadata().firstPage() != null) {
var pageRange = chunk.metadata().firstPage().equals(chunk.metadata().lastPage())
? "Page " + chunk.metadata().firstPage()
: "Pages " + chunk.metadata().firstPage() + "-" + chunk.metadata().lastPage();

            System.out.println("Chunk: " + chunk.text().substring(0, 50) +
                               "... (" + pageRange + ")");
        }
    }

}
