"""Element-aware node parser for xberg-extracted documents."""

import logging
import sys
from collections.abc import Sequence
from typing import Any

if sys.version_info >= (3, 12):
    from typing import override
else:
    from typing_extensions import override

from llama_index.core.node_parser import NodeParser
from llama_index.core.schema import BaseNode, Document, NodeRelationship, TextNode
from llama_index.core.utils import get_tqdm_iterable

logger = logging.getLogger(__name__)

_ELEMENT_METADATA_KEYS = ("element_type", "page_number", "element_index")

_MISSING_ELEMENTS_WARNING = (
    "Document %s has no '_xberg_elements' metadata. "
    "Passing through unchanged. Use XbergReader with "
    "ExtractionConfig(result_format='element_based') to populate element metadata."
)


class XbergNodeParser(NodeParser):
    """Element-aware node parser for xberg-extracted documents.

    Converts xberg-extracted document elements into individual TextNodes,
    preserving document structure (titles, headings, paragraphs, tables, etc.)
    through the RAG pipeline.

    Requires Documents produced by XbergReader with
    ``ExtractionConfig(result_format="element_based")``. Documents without
    ``_xberg_elements`` metadata are passed through unchanged with a warning.
    """

    @classmethod
    def class_name(cls) -> str:
        """Return the unique class identifier for serialisation."""
        return "XbergNodeParser"

    @override
    def _parse_nodes(
        self,
        nodes: Sequence[BaseNode],
        show_progress: bool = False,
        **kwargs: Any,
    ) -> list[BaseNode]:
        output: list[BaseNode] = []
        nodes_with_progress = get_tqdm_iterable(nodes, show_progress, "Parsing nodes")

        for node in nodes_with_progress:
            elements = node.metadata.get("_xberg_elements")

            if not isinstance(elements, list) or not elements:
                logger.warning(_MISSING_ELEMENTS_WARNING, node.node_id)
                output.append(node)
                continue

            source_ref = node.as_related_node_info()
            excluded_embed = list(node.excluded_embed_metadata_keys) + list(_ELEMENT_METADATA_KEYS)
            idx = 0

            for el in elements:
                text = el.get("text", "")
                if not text.strip():
                    continue

                el_meta = el.get("metadata", {})
                text_node = TextNode(
                    text=text,
                    id_=self.id_func(idx, node),
                    metadata={
                        "element_type": el.get("element_type", "unknown"),
                        "page_number": el_meta.get("page_number"),
                        "element_index": el_meta.get("element_index"),
                    },
                    excluded_embed_metadata_keys=excluded_embed,
                    excluded_llm_metadata_keys=list(node.excluded_llm_metadata_keys),
                    metadata_separator=node.metadata_separator,
                    metadata_template=node.metadata_template,
                    text_template=node.text_template,
                    relationships={NodeRelationship.SOURCE: source_ref},
                )
                output.append(text_node)
                idx += 1

        return output

    @staticmethod
    def _strip_elements_metadata(nodes: list[BaseNode]) -> list[BaseNode]:
        """Remove _xberg_elements from child TextNodes only.

        Passthrough documents keep their metadata untouched.
        """
        for node in nodes:
            if node.source_node is not None:
                node.metadata.pop("_xberg_elements", None)
        return nodes

    @override
    def _postprocess_parsed_nodes(self, nodes: list[BaseNode], parent_doc_map: dict[str, Document]) -> list[BaseNode]:
        nodes = super()._postprocess_parsed_nodes(nodes, parent_doc_map)
        return self._strip_elements_metadata(nodes)
