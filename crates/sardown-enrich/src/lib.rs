mod diagram;
mod highlight;
pub use diagram::{compile_diagrams, CompiledDiagram, DiagramTable};
pub use highlight::{ast_contains_code_block, Highlighter};
