use md2pdf_ast::{BlockNode, ImageSource};
use md2pdf_layout::decode_images;

#[test]
fn decodes_embedded_local_image_and_indexes_by_path() {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let ast = vec![BlockNode::Image { alt: "test".to_string(), title: None, source: ImageSource::Embedded(std::path::PathBuf::from("test-image.png")) }];
    let table = decode_images(&ast, &base_dir);
    let decoded = table.get("test-image.png").expect("image not found in table");
    assert_eq!((decoded.width, decoded.height), (2, 2));
    assert_eq!(decoded.rgba8.len(), 2 * 2 * 4);
}

#[test]
fn external_images_are_skipped_not_errored() {
    let ast = vec![BlockNode::Image { alt: "test".to_string(), title: None, source: ImageSource::External("https://example.com/pic.png".to_string()) }];
    let table = decode_images(&ast, std::path::Path::new("."));
    assert!(table.is_empty());
}
