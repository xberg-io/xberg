"""Package-specific exception classes."""


class SchemaNotInitializedError(RuntimeError):
    """Raised when ingestion is attempted before calling setup_schema()."""

    def __init__(self) -> None:
        super().__init__("Schema not initialized. Call setup_schema() before ingesting documents.")


class IngestionError(RuntimeError):
    """Raised when document ingestion fails due to a SurrealDB INSERT error."""

    def __init__(self, context: str, server_error: str) -> None:
        super().__init__(f"INSERT IGNORE failed silently during {context}: {server_error}")


class DimensionMismatchError(IngestionError):
    """Raised when vector dimensions conflict with an existing HNSW index.

    SurrealDB v3 enforces HNSW dimensions server-globally — once an index with
    dimension N exists anywhere on the server, inserts with a different dimension
    fail even across namespaces and databases. Use the same embedding model for
    all pipelines on the same server, or use separate SurrealDB instances.
    """

    def __init__(self, context: str, server_error: str) -> None:
        RuntimeError.__init__(
            self,
            f"Vector dimension mismatch during {context}. "
            "SurrealDB v3 enforces HNSW dimensions server-globally — "
            "once an index with dimension N exists anywhere on the server, "
            "inserts with a different dimension fail even across namespaces and databases. "
            "Use the same embedding model for all pipelines on the same server, "
            f"or use separate SurrealDB instances. Server error: {server_error}",
        )
