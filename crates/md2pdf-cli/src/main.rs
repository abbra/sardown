use clap::{Parser, Subcommand};
use md2pdf_ast::BlockNode;
use md2pdf_layout::{shape_paragraph, PositionedPage};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "md2pdf")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a Markdown file to PDF
    Render {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn build_font_db() -> anyhow::Result<fontdb::Database> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    Ok(db)
}

const PAGE_CONTENT_WIDTH_PT: f32 = 468.0; // 612pt page width minus 72pt margins each side

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Render { input, output } => {
            let markdown = std::fs::read_to_string(&input)?;
            let blocks = md2pdf_ast::parse(&markdown);

            let font_db = build_font_db()?;
            let mut font_system = cosmic_text::FontSystem::new_with_locale_and_db(
                "en-US".to_string(),
                font_db,
            );

            let mut elements = Vec::new();
            let mut y_cursor = 72.0f32;
            for block in &blocks {
                let content: &[md2pdf_ast::InlineNode] = match block {
                    BlockNode::Heading { content, .. } | BlockNode::Paragraph { content } => content,
                    _ => continue, // non-text blocks land in Phase 2's real layout pass
                };
                for mut el in shape_paragraph(&mut font_system, content, PAGE_CONTENT_WIDTH_PT) {
                    if let md2pdf_layout::PositionedElement::TextRun { y, .. } = &mut el {
                        *y += y_cursor;
                    }
                    elements.push(el);
                }
                y_cursor += 20.0; // fixed line-height spacer; real cursor management is Phase 2
            }

            let page = PositionedPage { page_number: 0, elements };
            let pdf_bytes = md2pdf_pdf::render_pdf(&[page], font_system.db())?;
            std::fs::write(&output, pdf_bytes)?;
            Ok(())
        }
    }
}
