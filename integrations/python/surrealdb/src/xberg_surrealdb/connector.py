"""Full-document extraction and BM25 search connector."""

from typing import Any, cast

from xberg import ExtractionResult

from xberg_surrealdb._base import BaseIngester, _check_insert_result, _map_result_to_doc
from xberg_surrealdb.schema import build_connector_schema


class DocumentConnector(BaseIngester):
    """Full-document extraction and BM25 search. No chunking or embedding."""

    ANALYZER_NAME: str = "doc_analyzer"

    @property
    def analyzer_name(self) -> str:
        """The BM25 analyzer name used in the schema."""
        return self.ANALYZER_NAME

    async def setup_schema(
        self,
        *,
        analyzer_language: str = "english",
        bm25_k1: float = 1.2,
        bm25_b: float = 0.75,
    ) -> None:
        """Create the documents table with BM25 index.

        Args:
            analyzer_language: Snowball stemmer language for the BM25 analyzer.
            bm25_k1: BM25 term-frequency saturation parameter.
            bm25_b: BM25 document-length normalization parameter.

        """
        stmts = build_connector_schema(
            table=self._table,
            analyzer_language=analyzer_language,
            bm25_k1=bm25_k1,
            bm25_b=bm25_b,
        )
        for stmt in stmts:
            await self._client.query(stmt)
        self._schema_ready = True

    async def _ingest_result(self, result: ExtractionResult, source: str) -> None:
        """Process a single extraction result.

        Args:
            result: The extraction result from Xberg.
            source: Identifier for the document origin (e.g. file path).

        """
        doc = _map_result_to_doc(result, source, self._table)
        res = await self._client.query(
            f"INSERT IGNORE INTO {self._table} $records",
            {"records": cast("Any", [doc])},
        )
        _check_insert_result(res, context="document insertion")
