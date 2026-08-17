use md2pdf_ast::{BlockNode, ImageSource};
use md2pdf_layout::{collect_svg_diagrams, decode_images};

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
fn decodes_embedded_jpeg_image() {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let ast = vec![BlockNode::Image { alt: "test".to_string(), title: None, source: ImageSource::Embedded(std::path::PathBuf::from("test-image.jpg")) }];
    let table = decode_images(&ast, &base_dir);
    let decoded = table.get("test-image.jpg").expect("jpeg image not found in table");
    assert_eq!((decoded.width, decoded.height), (3, 3));
    assert_eq!(decoded.rgba8.len(), 3 * 3 * 4);
}

#[test]
fn path_traversal_outside_base_dir_is_rejected() {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    // Escapes tests/fixtures/ back up to the crate root and reads Cargo.toml — must be refused
    // even though the file exists and is a valid target for `image::open` to attempt.
    let ast = vec![BlockNode::Image { alt: "traversal".to_string(), title: None, source: ImageSource::Embedded(std::path::PathBuf::from("../../Cargo.toml")) }];
    let table = decode_images(&ast, &base_dir);
    assert!(table.is_empty(), "path traversal outside base_dir must not decode anything");
}

#[test]
fn absolute_path_outside_base_dir_is_rejected() {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    // An absolute path would make Path::join discard base_dir entirely if not guarded against.
    let ast = vec![BlockNode::Image {
        alt: "absolute".to_string(),
        title: None,
        source: ImageSource::Embedded(std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))),
    }];
    let table = decode_images(&ast, &base_dir);
    assert!(table.is_empty(), "absolute path outside base_dir must not decode anything");
}

#[test]
fn collects_an_embedded_svg_with_its_intrinsic_size() {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let ast = vec![BlockNode::Image { alt: "test".to_string(), title: None, source: ImageSource::Embedded(std::path::PathBuf::from("test-vector.svg")) }];
    let table = collect_svg_diagrams(&ast, &base_dir);
    let diagram = table.get("test-vector.svg").expect("svg not found in table");
    assert_eq!((diagram.width, diagram.height), (100.0, 50.0));
    assert!(diagram.svg.contains("<svg"));
}

#[test]
fn an_svg_image_is_absent_from_the_raster_image_table() {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let ast = vec![BlockNode::Image { alt: "test".to_string(), title: None, source: ImageSource::Embedded(std::path::PathBuf::from("test-vector.svg")) }];
    let table = decode_images(&ast, &base_dir);
    assert!(table.is_empty(), "expected an .svg file to be left for collect_svg_diagrams, not decoded as a raster image");
}

#[test]
fn svg_path_traversal_outside_base_dir_is_rejected() {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    // Escapes tests/fixtures/ back up to tests/secret.svg -- a real, valid, .svg-extensioned file
    // that exists and would otherwise be a valid target for the SVG collection path.
    let ast = vec![BlockNode::Image { alt: "traversal".to_string(), title: None, source: ImageSource::Embedded(std::path::PathBuf::from("../secret.svg")) }];
    let table = collect_svg_diagrams(&ast, &base_dir);
    assert!(table.is_empty(), "path traversal outside base_dir must not be read");
}

#[test]
fn external_images_are_skipped_not_errored() {
    let ast = vec![BlockNode::Image { alt: "test".to_string(), title: None, source: ImageSource::External("https://example.com/pic.png".to_string()) }];
    let table = decode_images(&ast, std::path::Path::new("."));
    assert!(table.is_empty());
}

#[test]
fn decode_images_recurses_into_columns() {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let ast = vec![BlockNode::Columns(vec![vec![BlockNode::Image {
        alt: "test".to_string(),
        title: None,
        source: ImageSource::Embedded(std::path::PathBuf::from("test-image.png")),
    }]])];
    let table = decode_images(&ast, &base_dir);
    assert!(table.contains_key("test-image.png"), "expected the image inside the column to be decoded");
}

#[test]
fn collect_svg_diagrams_recurses_into_columns() {
    let base_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let ast = vec![BlockNode::Columns(vec![vec![BlockNode::Image {
        alt: "test".to_string(),
        title: None,
        source: ImageSource::Embedded(std::path::PathBuf::from("test-vector.svg")),
    }]])];
    let table = collect_svg_diagrams(&ast, &base_dir);
    assert!(table.contains_key("test-vector.svg"), "expected the SVG inside the column to be collected");
}
