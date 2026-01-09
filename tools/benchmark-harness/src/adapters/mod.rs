//! Framework adapter implementations

pub mod external;
pub mod kreuzberg;
pub mod native;
pub mod node;
pub mod python;
pub mod ruby;
pub mod subprocess;

pub use external::{
    create_docling_adapter, create_docling_batch_adapter, create_markitdown_adapter, create_mineru_adapter,
    create_mineru_batch_adapter, create_pandoc_adapter, create_pdfplumber_adapter, create_pdfplumber_batch_adapter,
    create_pymupdf4llm_adapter, create_tika_adapter, create_tika_batch_adapter, create_unstructured_adapter,
};
pub use kreuzberg::{
    create_csharp_adapter, create_csharp_batch_adapter, create_elixir_adapter, create_elixir_batch_adapter,
    create_go_adapter, create_go_batch_adapter, create_java_adapter, create_java_batch_adapter, create_node_adapter,
    create_node_batch_adapter, create_php_adapter, create_php_batch_adapter, create_python_adapter,
    create_python_batch_adapter, create_ruby_adapter, create_ruby_batch_adapter, create_wasm_adapter,
    create_wasm_batch_adapter,
};
pub use native::NativeAdapter;
pub use node::NodeAdapter;
pub use python::PythonAdapter;
pub use ruby::RubyAdapter;
pub use subprocess::SubprocessAdapter;
