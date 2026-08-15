use md2pdf_ast::{BlockNode, ImageSource};
use std::collections::HashMap;
use std::path::Path;

pub struct DecodedImage {
    pub rgba8: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub type ImageTable = HashMap<String, DecodedImage>;

pub fn decode_images(ast: &[BlockNode], base_dir: &Path) -> ImageTable {
    let mut table = HashMap::new();
    collect(ast, base_dir, &mut table);
    table
}

fn collect(ast: &[BlockNode], base_dir: &Path, table: &mut ImageTable) {
    for block in ast {
        match block {
            BlockNode::Image { source: ImageSource::Embedded(path), .. } => {
                let key = path.to_string_lossy().to_string();
                if table.contains_key(&key) {
                    continue;
                }
                match image::open(base_dir.join(path)) {
                    Ok(img) => {
                        let rgba = img.to_rgba8();
                        table.insert(key, DecodedImage { width: rgba.width(), height: rgba.height(), rgba8: rgba.into_raw() });
                    }
                    Err(e) => eprintln!("warning: failed to decode image {key}: {e}"),
                }
            }
            BlockNode::Image { source: ImageSource::External(url), .. } => {
                eprintln!("warning: skipping external image (not fetched): {url}");
            }
            BlockNode::Blockquote { content } => collect(content, base_dir, table),
            BlockNode::List { items, .. } => {
                for item in items {
                    collect(item, base_dir, table);
                }
            }
            _ => {}
        }
    }
}
