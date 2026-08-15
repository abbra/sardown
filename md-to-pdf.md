# **Native Rust Markdown-to-PDF Engine: Architecture & Extended Development Plan**

---

This document details the software architecture, core component designs, data flows, detailed Rust data structures, design rationale, extended development roadmap, testing strategy, and performance targets for a pure Rust direct Markdown-to-PDF rendering engine.

## **1\. Executive Summary & Goals**

The goal of this project is to create a high-performance, single-binary CLI tool and library written in Rust that transforms Markdown documents into production-quality PDF files. Key requirements include support for rich text layout, tables, code syntax highlighting, internal cross-linking, and dynamic diagram generation (Mermaid to vector PDF).

## **2\. System Architecture & Component Design**

The system operates as a directional processing pipeline. Raw Markdown text flows into an AST parser, is converted into a structured document model, moves through layout calculation and pagination engines, and finally outputs binary PDF streams via low-level primitives.

\+---------------------+     \+----------------------+     \+-----------------------+  
|  Markdown Source    | \--\> |  pulldown-cmark      | \--\> |  Structural AST Node  |  
|  Document (.md)     |     |  Event Stream        |     |  Tree Conversion      |  
\+---------------------+     \+----------------------+     \+-----------------------+  
                                                                     |  
                                                                     v  
\+---------------------+     \+----------------------+     \+-----------------------+  
| PDF File Output     | \<-- | krilla                | \<-- | Layout & Pagination   |  
| (.pdf stream)       |     | (PDF/A-validated,     |     | Engine (cosmic-text)  |  
|                      |     |  wraps pdf-writer,    |     |                       |  
|                      |     |  subsetter)           |     |                       |  
\+---------------------+     \+----------------------+     \+-----------------------+  
                                                                     ^  
                                                                     |  
                                             \+-----------------------+-----------------------+  
                                             |                                               |  
                                 \+-----------------------+                       \+-----------------------+  
                                 |  Native Mermaid       |                       | Syntect Highlighting  |  
                                 |  Renderer (merman)    |                       | Engine                |  
                                 \+-----------------------+                       \+-----------------------+

### **2.1 Core Subsystems Overview**

| Subsystem | Primary Responsibility | Key Rust Dependencies   |
| :---- | :---- | :---- |
| **Parser & Intermediate Representation** | Converts raw Markdown stream into a rich, structural document tree. Intercepts special blocks (code, diagrams). | pulldown-cmark |
| **Diagram & Syntax Processing** | Renders Mermaid blocks to SVG using a native Rust diagram engine (no embedded JS/DOM) and highlights code syntax into styled text runs. | syntect, merman, usvg |
| **Layout & Typography Engine** | Performs text shaping, line wrapping, font fallback, auto-sizing, and pagination across page geometry. | cosmic-text |
| **Cross-Reference & Link Resolver** | Tracks anchors, targets, headings, and internal links in a two-pass resolution process. | Custom Rust module |
| **PDF Vector Stream Generator** | Emits PDF dictionaries, character subsets, font objects, vector paths, page content streams, embedded diagram SVGs, and PDF/A metadata (XMP, OutputIntent/ICC). | krilla (wraps pdf-writer, subsetter, rustybuzz, skrifa), krilla-svg |

## **3\. Key Data Structure Definitions & Design Rationales**

Below are the core Rust types used to represent the Intermediate AST and Layout Tree across the rendering pipeline, alongside the structural design rationale.

// Intermediate AST Representation  
pub enum BlockNode {  
    Heading { level: u8, id: String, content: Vec<InlineNode> },  
    Paragraph { content: Vec<InlineNode> },  
    CodeBlock { language: Option<String>, tokens: Vec<HighlightedToken> },  
    Blockquote { content: Vec<BlockNode> },  
    ThematicBreak,  
    MermaidDiagram { id: String, source: String },  
    Image { alt: String, title: Option<String>, source: ImageSource },  
    Table { headers: Vec<InlineNode>, rows: Vec<Vec<InlineNode>>, alignments: Vec<ColumnAlignment> },  
    List { ordered: bool, items: Vec<Vec<BlockNode>> },  
}

pub enum ImageSource {  
    Embedded(PathBuf),  
    External(String),  
}

pub struct InlineNode {  
    pub text: String,  
    pub style: TextStyle,  
    pub link\_target: Option<LinkTarget>,  
}

pub enum LinkTarget {  
    InternalAnchor(String),  
    ExternalUrl(String),  
}

// Side-table populated by the diagram compilation pre-pass, keyed by  
// MermaidDiagram::id — keeps BlockNode free of post-parse mutable state,  
// mirroring the cross-reference symbol map in 4.3.  
pub type DiagramTable = HashMap<String, CompiledDiagram>;

pub struct CompiledDiagram {  
    pub svg: String,  
    pub width: f32,  
    pub height: f32,  
}

// Layout & Geometry Outputs  
pub struct PositionedPage {  
    pub page\_number: usize,  
    pub elements: Vec<PositionedElement>,  
}

pub enum PositionedElement {  
    TextRun { x: f32, y: f32, glyphs: Vec<PositionedGlyph>, font\_id: String, size: f32, color: \[u8; 3\] },  
    Path { points: Vec<PathCommand>, fill: Option<\[u8; 3\]>, stroke: Option<StrokeStyle> },  
    VectorGraphic { x: f32, y: f32, width: f32, height: f32, diagram\_id: String },  
    LinkAnnotation { rect: Rect, destination: LinkTarget },  
}

### **3.1 Design Choices: BlockNode (Semantic AST)**

* **Pre-processed Tokenization over Raw Strings:**  
  * CodeBlock stores Vec\<HighlightedToken\> rather than raw string slices. Highlighting with syntect runs as a distinct enrichment pass *after* parsing (not inside the parser itself), so the parser stays decoupled from theme/color-scheme selection and a future light/dark theme toggle only re-runs this pass, not the parse.  
  * MermaidDiagram stores only an `id` and the raw `source` string — it holds no compiled output. Diagram compilation (native Mermaid rendering to SVG via merman) runs as its own pre-pass over the AST and writes results into a `DiagramTable` side-table keyed by `id`, mirroring the cross-reference symbol map described in 4.3. This keeps `BlockNode` immutable once constructed, rather than mutating an AST node in place after parsing.  
* **Hierarchy Flattener for Inlines:** Block elements hold Vec\<InlineNode\> (styled text runs or links), keeping structural elements (lists, headers, paragraphs, blockquotes) isolated from line-shaping logic.  
* **Decoupled Geometry:** BlockNode contains zero coordinate or sizing data (no widths, heights, margins, or page indices). This guarantees that AST construction remains independent of paper size (A4 vs. US Letter) or margin configurations.

### **3.2 Design Choices: PositionedElement (Fixed Vector Canvas)**

* **Post-Shaped Glyph Positioning (GlyphLayout):** TextRun does not store raw strings; it stores individual glyph IDs and horizontal advances computed by cosmic-text (exposed per-glyph via `LayoutGlyph`: font ID, glyph ID, position, and cluster range). By the time krilla receives this struct, it does not perform font shaping or line wrapping—it emits Tj (Show Text) operators for resolved glyph indices and owns subsetting/embedding. Note: outline-only glyphs take this path directly; COLR/SVG-in-OT/bitmap glyphs are not outline-only and require a Type 3 font fallback, which needs explicit handling rather than being assumed to "just work" through the same TextRun path. ToUnicode generation for ligatures/multi-codepoint clusters must be derived from cosmic-text's cluster start/end fields and needs its own test coverage.  
* **Unified Vector Primitives (Path):** Tables (grid lines, cell fills), blockquotes (side borders), and code block background boxes all lower down to Path variants (PathCommand::MoveTo, PathCommand::LineTo, PathCommand::CubicTo). This reduces the final PDF generator stage to a simple canvas drawing loop. Embedded Mermaid diagrams are deliberately **not** decomposed into Path primitives — they use the separate `VectorGraphic` variant, which references a `DiagramTable` entry by ID. The PDF generator hands that entry's SVG straight to `krilla-svg`'s `draw_svg(tree, size, ...)`, which draws a parsed `usvg::Tree` onto the krilla surface directly. Re-decomposing an arbitrary SVG into our own Path primitives would mean reimplementing SVG rendering; krilla-svg already does this correctly.  
* **Explicit Spatial Annotations (LinkAnnotation):** In PDF standards, clickable internal or external hyperlinks are separate overlay metadata objects (/Annot dictionaries) tied to explicit rectangular bounding boxes (Rect) rather than inline text attributes. Separating LinkAnnotation into its own PositionedElement variant allows Pass 2 of the cross-reference engine to overlay link areas over laid-out text coordinates seamlessly.

## **4\. Subsystem Specifications**

### **4.1 Intermediate AST Processing**

The parser transforms pulldown-cmark events into strongly typed Rust structural blocks. This abstraction decouples parsing logic from rendering passes:

* **Text Nodes:** Rich text runs containing font weight, slants, sizes, color, and hyperlinks.  
* **Code Blocks:** Enriched via syntect (as a post-parse pass, see 3.1) to inject background styling and tokenized foreground colors.  
* **Mermaid Diagrams:** Compiled by merman, a native Rust Mermaid renderer, directly to SVG — no JavaScript engine or DOM is involved. The resulting SVG string and its intrinsic width/height are stored in the `DiagramTable` side-table keyed by diagram ID (3.1); at PDF-emission time the SVG is parsed into a `usvg::Tree` and drawn directly onto the krilla surface via `krilla-svg` (3.2) — no separate SVG-to-PDF-path conversion crate is needed. This deliberately avoids embedding a JS engine: Mermaid.js itself requires a live browser DOM for text-metric calls (`getBBox`/`getComputedTextLength`), which is why tools like mermaid-cli rely on headless Chrome rather than a lightweight JS runtime — a dependency this design explicitly avoids.  
* **Images:** Referenced via `BlockNode::Image`; embedded (local path) or external images are decoded and placed as raster PDF XObjects during layout.  
* **Blockquotes:** Nested `BlockNode` content indented and rendered with a side-border Path (3.2).  
* **Tables:** Structured grid models holding column alignment constraints, header properties, and cell spans.

### **4.2 Layout, Typography & Pagination Algorithm**

Layout calculations convert abstract elements into printable, millimeter-precise coordinates on a target page canvas:

1. **Page Geometry:** Defines margins, printable boundary vectors (width x height), and header/footer reservation areas. Multi-column layout is out of scope for this design — single-column flow only.  
2. **Text Shaping & Bidi Support:** cosmic-text evaluates complex scripts, kerning, font fallbacks, and calculates exact line boundaries.  
3. **Page Cursor Management:** Tracks vertical Y-coordinates. When an element height exceeds remaining page space, page breaks are triggered.  
4. **Widow & Orphan Controls:** Headings enforce keep-with-next rules, preventing isolated headings at page footers. Paragraphs enforce minimum two-line splits.  
5. **Table Auto-sizing:** Two-pass width calculation measures minimum content boundaries and preferred widths to proportionally scale columns within printable margins.

### **4.3 Two-Pass Cross-Reference System**

Because target page numbers for internal links (e.g., \[See Chapter 2\](\#chap-2)) are unknown until layout completes, rendering uses two distinct passes:

* **Pass 1 (Layout & Mapping):** Traverses the AST, computes coordinates, assigns target anchors to page numbers, and builds a symbol map of target IDs to (PageNum, X, Y) coordinates.  
* **Pass 2 (PDF Generation):** Writes PDF object streams. Any anchor references pull exact target coordinates and page object references from the symbol map, creating PDF /Link annotation dictionaries.

Cross-references in this design are **clickable `/Link` annotations only** — resolved link text never includes the target's page number inline (e.g. no "see page 42" substitution). This is a deliberate scope boundary: textual page-number substitution would require a reflow fixpoint loop (inserted text changes line wrapping, which can change page numbers, which changes the inserted text), which this two-pass design does not attempt.

Heading anchor IDs are generated by slugifying heading text (lowercase, non-alphanumeric characters replaced with `-`, collapsed/trimmed), with a numeric suffix (`-1`, `-2`, ...) appended on collision within a document.

### **4.4 PDF/A Conformance & Validation**

PDF/A conformance is a first-class subsystem, not a byproduct of using `pdf-writer`. This design targets **PDF/A-2b**, chosen over PDF/A-1b (which forbids transparency, ruling out translucent diagram fills) and over the accessibility-tagged `-a` variants (out of scope for this project). Achieving it requires:

* **krilla's validated PDF/A-2b export mode**, which owns XMP metadata generation (kept in sync with the legacy Info dictionary), `OutputIntent` dictionary construction with an embedded ICC profile, and PDF/A-legal font subsetting (required table retention, correct cmap/CID mapping) — all of which `pdf-writer` alone provides no support for.  
* **A hard ban enforced at the API boundary** on the constructs PDF/A forbids: no embedded JavaScript, no encryption, no external file references, no LZW-compressed streams.  
* **veraPDF-based validation in CI** (6.), since PDF/A conformance is only meaningfully demonstrated by validator output, not by spec review.

## **5\. Detailed Implementation Plan & Workstreams**

| Phase & Workstream | Sprint Objectives & Milestones | Technical Deliverables | Duration   |
| :---- | :---- | :---- | :---- |
| **Phase 1: Core Parsing & Basic Text** | Setup Rust workspace & CLI runner Implement Markdown to AST lowering Single-page line wrapping & font loading, bridging cosmic-text's `LayoutGlyph` output into krilla's `Surface::draw_glyphs`/`KrillaGlyph` (verified compatible, see 3.2) | pulldown-cmark transformer module Basic cosmic-text shaper pipeline Simple krilla-based single page generator | Weeks 1 \- 3 |
| **Phase 2: Layout Engine & Advanced Formatting** | Multi-page flow & page cursor management Syntax highlighting as a post-parse enrichment pass Table auto-sizing and border grid layout | Pagination engine with widow/orphan protection syntect highlighting enrichment pass Two-pass dynamic table layout algorithm | Weeks 4 \- 6 |
| **Phase 3: Diagrams & Linking** | Native Mermaid compilation via merman \+ DiagramTable pre-pass Internal cross-references & TOC links Vector graphic placement via krilla-svg | merman diagram pipeline (no JS engine) Two-pass link symbol table resolver PDF /Annot link dictionary builder | Weeks 7 \- 10 |
| **Phase 4: Font Subsetting & PDF/A Conformance** | OpenType font character subsetting via krilla PDF/A-2b export: XMP metadata, OutputIntent/ICC profile embedding veraPDF-validated CI conformance gate | krilla-based subsetting/embedding pipeline PDF/A-2b export mode wired end-to-end veraPDF CI gate (blocks merge on conformance failure) | Weeks 11 \- 13 |
| **Phase 5: Production Hardening** | Visual regression test suite Benchmarking & performance optimization | PDF rasterization & visual snapshot tests Stream-based PDF emission optimizations | Week 14 |

## **6\. Quality Assurance & Testing Strategy**

To guarantee layout stability and visual accuracy across software releases, the project employs a four-tier testing strategy:

* **Unit Tests:** Validate AST conversion logic, table cell width calculations, and two-pass link resolving.  
* **Visual Regression Testing:** Converts output PDFs into PNG images using pdfium-render and performs pixel-diff comparisons against ground-truth golden snapshots. (pdfium-render is a dev/test-only dependency and does not affect the shipped binary's zero-external-dependency requirement, 7.)  
* **Property-Based Testing:** Generates randomized Markdown documents using proptest to verify that layout algorithms never panic or cause buffer overflows regardless of input structure.  
* **PDF/A Conformance Validation:** Every generated PDF is validated against veraPDF's PDF/A-2b ruleset in CI (4.4); conformance failures block merge.

## **7\. Non-Functional Requirements & Performance Targets**

* **Binary Size:** Single standalone executable under 25MB with zero external system dependencies (no Node.js, Python, or LaTeX runtimes required).  
* **Compilation Speed:** Process and render a 200-page Markdown technical book (including diagrams and code blocks) in under 1.5 seconds.  
* **Memory Footprint:** Stream-based page creation ensuring peak memory usage remains under 150MB during large book renders.  
* **PDF Conformance:** Produce **PDF/A-2b** compliant documents (XMP metadata, embedded OutputIntent/ICC profile, fully embedded font subsets, no forbidden constructs) validated in CI via veraPDF (6., 4.4), suitable for long-term archiving and professional printing.