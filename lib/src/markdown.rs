use crate::core::Element::{Header, Hyperlink,  List, Table, Text};
use crate::core::*;
use bytes::Bytes;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd, TextMergeStream};
use std::cell::RefCell;
use comrak::Arena;
use comrak::arena_tree::Node;

pub struct Transformer;

impl TransformerTrait for Transformer {
    fn parse(document: &Bytes) -> anyhow::Result<Document> {
        Transformer::parse_with_loader(document, disk_image_loader("."))
    }

    fn generate(document: &Document) -> anyhow::Result<Bytes> {
        Transformer::generate_with_saver(document, disk_image_saver("."))
    }
}

struct ImageSaver<F> where F: Fn(&Bytes, &str) -> anyhow::Result<()> {
    pub function: F,
}
impl TransformerWithImageLoaderSaverTrait for Transformer {
    fn parse_with_loader<F>(document: &Bytes, image_loader: F) -> anyhow::Result<Document>
        where F: Fn(&str) -> anyhow::Result<Bytes>,Self: Sized,
    {
        fn process_element_creation(
            current_element: &mut Option<Element>,
            el: Element,
            list_depth: i32,
        ) {
            match current_element {
                Some(element) => match element {
                    Element::List { elements, .. } => {
                        let mut li_vec_to_insert = elements;

                        for _ in 1..list_depth {
                            let last_index = li_vec_to_insert.len() - 1;
                            if let Element::List {
                                elements: ref mut inner_els,
                                ..
                            } = li_vec_to_insert[last_index].element
                            {
                                li_vec_to_insert = inner_els;
                            } else {
                                panic!("Expected a nested list structure at the specified depth");
                            }
                        }

                        match &el {
                            Element::Hyperlink { .. } | Element::Header { .. } => {
                                if let Some(ListItem { element }) = li_vec_to_insert.last() {
                                    if let Text { .. } = element {
                                        li_vec_to_insert.pop();
                                    }
                                }
                            }

                            _ => {}
                        }

                        let li = ListItem { element: el };
                        li_vec_to_insert.push(li);
                    }
                    _ => {}
                },
                None => {
                    *current_element = Some(el);
                }
            }
        }

        let document_str = std::str::from_utf8(document)?;
        let mut elements: Vec<Element> = Vec::new();

        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
        options.insert(Options::ENABLE_MATH);
        options.insert(Options::ENABLE_GFM);

        let parser = Parser::new_ext(document_str, options);
        let md_iterator = TextMergeStream::new(parser);

        let mut list_depth = 0;
        let mut current_element: Option<Element> = None;

        let mut table_element: Option<(bool, Element)> = None;
        for event in md_iterator {
            match event {
                Event::Start(tag) => {
                    match tag {
                        Tag::Paragraph => {
                            process_element_creation(
                                &mut current_element,
                                Element::Paragraph { elements: vec![] },
                                list_depth,
                            );
                        }
                        Tag::Heading { level, .. } => {
                            let level = match level {
                                HeadingLevel::H1 => 1,
                                HeadingLevel::H2 => 2,
                                HeadingLevel::H3 => 3,
                                HeadingLevel::H4 => 4,
                                HeadingLevel::H5 => 5,
                                HeadingLevel::H6 => 6,
                            };
                            process_element_creation(
                                &mut current_element,
                                Element::Header {
                                    level,
                                    text: "".to_string(),
                                },
                                list_depth,
                            );
                        }
                        Tag::List(numbered) => {
                            let numbered = numbered.is_some();

                            let list_el = List {
                                elements: vec![],
                                numbered,
                            };

                            process_element_creation(&mut current_element, list_el, list_depth);
                            list_depth += 1;
                        }
                        Tag::Item => {
                            let list_li = Text {
                                text: "".to_string(),
                                size: 14,
                            };

                            process_element_creation(&mut current_element, list_li, list_depth);
                        }
                        Tag::Table(_) => {
                            let table_el = Table {
                                headers: vec![],
                                rows: vec![],
                            };

                            table_element = Some((false, table_el));
                        }
                        Tag::TableHead => {
                            if let Some(table) = table_element.as_mut() {
                                table.0 = true;
                            }
                        }
                        Tag::Image {
                            dest_url, title, ..
                        } => {
                            let img_type = dest_url.to_string();
                            let bytes = image_loader(&dest_url)?;
                            let img_el = Element::Image(ImageData::new(
                                bytes,
                                title.to_string(),
                                title.to_string(),
                                img_type,
                                "".to_string(),
                                ImageDimension::default()
                            ));
                            // Before image there is paragraph tag (likely because alt text is in paragraph )
                            current_element = None;
                            process_element_creation(&mut current_element, img_el, list_depth);
                        }
                        Tag::Link {
                            dest_url, title, ..
                        } => {
                            let link_element = Hyperlink {
                                title: title.to_string(),
                                url: dest_url.to_string(),
                                alt: "alt".to_string(),
                                size: 14,
                            };
                            process_element_creation(
                                &mut current_element,
                                link_element,
                                list_depth,
                            );
                        }

                        _rest => {
                            // println!("The tag parsing is not implemented {:#?}", rest);
                        }
                    }
                }
                Event::Text(text) => {
                    if let Some(curr_el) = current_element.as_mut() {
                        match curr_el {
                            Element::Paragraph { ref mut elements } => {
                                elements.push(Element::Text {
                                    text: text.to_string(),
                                    size: 14,
                                })
                            }
                            Element::Header { text: el_text, .. } => {
                                el_text.push_str(&text);
                            }
                            Element::List { elements, .. } => {
                                let mut li_vec_to_insert = elements;

                                for _ in 1..list_depth {
                                    let last_index = li_vec_to_insert.len() - 1;
                                    if let Element::List {
                                        elements: ref mut inner_els,
                                        ..
                                    } = li_vec_to_insert[last_index].element
                                    {
                                        li_vec_to_insert = inner_els;
                                    } else {
                                        panic!("Expected a nested list structure at the specified depth");
                                    }
                                }

                                let li = li_vec_to_insert.last_mut().unwrap();

                                match &mut li.element {
                                    Text {
                                        text: element_text, ..
                                    } => {
                                        element_text.push_str(&text);
                                    }
                                    Hyperlink { title, .. } => {
                                        *title = text.to_string();
                                    }
                                    Header {
                                        text: header_text, ..
                                    } => {
                                        *header_text = text.to_string();
                                    }
                                    _ => {}
                                }
                            }
                            Element::Image(image) => {
                                image.set_image_alt(&text)
                            }
                            Element::Hyperlink {
                                alt,
                                ..
                            } => {
                                *alt = alt.to_string();
                            }
                            _ => {}
                        }
                    }
                    match table_element {
                        Some(ref mut t_el) => {
                            if let (is_header, Element::Table { headers, rows }) = t_el {
                                if *is_header {
                                    headers.push(TableHeader {
                                        element: Text {
                                            text: text.to_string(),
                                            size: 14,
                                        },
                                        width: 30.,
                                    })
                                } else {
                                    let last_row = rows.last_mut();

                                    match last_row {
                                        Some(tr) => {
                                            if tr.cells.len() == headers.len() {
                                                rows.push(TableRow {
                                                    cells: vec![TableCell {
                                                        element: Text {
                                                            text: text.to_string(),
                                                            size: 14,
                                                        },
                                                    }],
                                                });
                                            } else {
                                                tr.cells.push(TableCell {
                                                    element: Text {
                                                        text: text.to_string(),
                                                        size: 14,
                                                    },
                                                });
                                            }
                                        }
                                        None => {
                                            rows.push(TableRow {
                                                cells: vec![TableCell {
                                                    element: Text {
                                                        text: text.to_string(),
                                                        size: 14,
                                                    },
                                                }],
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        None => {}
                    }
                }
                Event::End(tag) => match tag {
                    TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Link | TagEnd::Image => {
                        let curr_el = current_element.take();
                        if let Some(curr_el) = curr_el {
                            match curr_el {
                                List { .. } => current_element = Some(curr_el),
                                _ => {
                                    elements.push(curr_el);
                                }
                            }
                        }
                    }
                    TagEnd::List(_) => {
                        list_depth -= 1;

                        if list_depth == 0 {
                            let curr_el = current_element.take();
                            if let Some(curr_el) = curr_el {
                                elements.push(curr_el);
                            }
                        }
                    }
                    TagEnd::TableHead => {
                        if let Some((is_header, _t_el)) = &mut table_element {
                            *is_header = false;
                        }
                    }
                    TagEnd::Table => {
                        if let Some((_, t_el)) = table_element.take() {
                            elements.push(t_el);
                        }
                    }
                    _ => {}
                },

                _ => {}
            }
        }

        Ok(Document::new(elements))
    }

    fn generate_with_saver<F>(document: &Document, image_saver: F) -> anyhow::Result<Bytes>
        where
            F: Fn(&Bytes, &str) -> anyhow::Result<()>,
    {
        use comrak::{format_commonmark, Arena, Options};
        use std::cell::RefCell;
        use comrak::nodes::LineColumn;

        let arena = Arena::new();

        let root = arena.alloc(Node::new(RefCell::new(Ast::new(
            NodeValue::Document,
            LineColumn { line: 0, column: 0 },
        ))));

        let image_num = RefCell::new(0);

        let image_saver = ImageSaver {
            function: &image_saver,
        };

        let all_elements: Vec<&Element> = document
            .page_header
            .iter()
            .chain(document.elements.iter())
            .chain(document.page_footer.iter())
            .collect();

        for element in &all_elements {
            let node = element_to_ast_node(&arena, element, &image_num, &image_saver)?;
            root.append(node);
        }

        let mut md = vec![];
        format_commonmark(root, &Options::default(), &mut md)?;

        Ok(Bytes::from(md))
    }
}

use comrak::nodes::{
    Ast, AstNode, LineColumn, NodeHeading, NodeLink, NodeList, NodeTable, NodeValue, TableAlignment,
};

fn element_to_ast_node<'a, F>(
    arena: &'a Arena<AstNode<'a>>,
    element: &Element,
    image_num: &RefCell<i32>,
    image_saver: &ImageSaver<F>,
) -> anyhow::Result<&'a AstNode<'a>>
    where
        F: Fn(&Bytes, &str) -> anyhow::Result<()>,
{
    match element {
        Element::Text { text, .. } => {
            let node = arena.alloc(Node::new(RefCell::new(Ast::new(
                NodeValue::Text(text.clone()),
                LineColumn { line: 0, column: 0 },
            ))));
            Ok(node)
        }

        Element::Header { level, text } => {
            let heading = arena.alloc(Node::new(RefCell::new(Ast::new(
                NodeValue::Heading(NodeHeading {
                    level: *level as u8,
                    setext: false,
                }),
                LineColumn { line: 0, column: 0 },
            ))));
            let text_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                NodeValue::Text(text.clone()),
                LineColumn { line: 0, column: 0 },
            ))));
            heading.append(text_node);
            Ok(heading)
        }

        Element::Paragraph { elements } => {
            let paragraph = arena.alloc(Node::new(RefCell::new(Ast::new(
                NodeValue::Paragraph,
                LineColumn { line: 0, column: 0 },
            ))));
            for child_element in elements {
                let child_node =
                    element_to_ast_node(arena, child_element, image_num, image_saver)?;
                paragraph.append(child_node);
            }
            Ok(paragraph)
        }

        Element::List { elements, numbered } => {
            use comrak::nodes::{ListDelimType, ListType};
            let list_type = if *numbered {
                ListType::Ordered
            } else {
                ListType::Bullet
            };

            let list_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                NodeValue::List(NodeList {
                    list_type,
                    start: if *numbered { 1 } else { 0 },
                    delimiter: ListDelimType::Period,
                    bullet_char: b'-',
                    tight: true,
                    marker_offset: 0,
                    padding: 2,
                }),
                LineColumn { line: 0, column: 0 },
            ))));

            for list_item in elements {
                let item_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                    NodeValue::Item(NodeList {
                        tight: true,
                        ..Default::default()
                    }),
                    LineColumn { line: 0, column: 0 },
                ))));

                let child_node = element_to_ast_node(arena, &list_item.element, image_num, image_saver)?;
                if matches!(&child_node.data.borrow().value, NodeValue::List(_)) {
                    // For nested lists, directly append the list node to the item
                    item_node.append(child_node);
                } else {
                    // For non-list items, ensure they are wrapped in a paragraph if not already
                    if !matches!(&child_node.data.borrow().value, NodeValue::Paragraph) {
                        let paragraph_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                            NodeValue::Paragraph,
                            LineColumn { line: 0, column: 0 },
                        ))));
                        paragraph_node.append(child_node);
                        item_node.append(paragraph_node);
                    } else {
                        item_node.append(child_node);
                    }
                }

                list_node.append(item_node);
            }
            Ok(list_node)
        }


        Element::Image(image_data) => {
            *image_num.borrow_mut() += 1;
            let image_extension = image_data.image_type().to_extension();
            let image_filename = format!("image{}{}", image_num.borrow(), image_extension);

            (image_saver.function)(image_data.bytes(), &image_filename)?;

            let image_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                NodeValue::Image(NodeLink {
                    url: image_filename.clone(),
                    title: image_data.title().to_string(),
                }),
                LineColumn { line: 0, column: 0 },
            ))));

            let paragraph_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                NodeValue::Paragraph,
                LineColumn { line: 0, column: 0 },
            ))));
            paragraph_node.append(image_node);

            Ok(paragraph_node)
        }


        Element::Hyperlink {
            title, url, alt, ..
        } => {
            let link_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                NodeValue::Link(NodeLink {
                    url: url.clone(),
                    title: alt.clone(),
                }),
                LineColumn { line: 0, column: 0 },
            ))));
            let text_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                NodeValue::Text(title.clone()),
                LineColumn { line: 0, column: 0 },
            ))));
            link_node.append(text_node);
            Ok(link_node)
        }

        Element::Table { headers, rows } => {
            let num_columns = headers.len() as u32;
            let num_rows = rows.len() as u32 + 1;

            let alignments = vec![TableAlignment::None; num_columns as usize];

            let table_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                NodeValue::Table(NodeTable {
                    alignments,
                    num_columns: num_columns as usize,
                    num_rows: num_rows as usize,
                    num_nonempty_cells: 0, // Adjust as needed
                }),
                LineColumn { line: 0, column: 0 },
            ))));

            // Header row
            let header_row_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                NodeValue::TableRow(true), // Indicate header row
                LineColumn { line: 0, column: 0 },
            ))));
            for header in headers {
                let cell_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                    NodeValue::TableCell,
                    LineColumn { line: 0, column: 0 },
                ))));
                let cell_content =
                    element_to_ast_node(arena, &header.element, image_num, image_saver)?;
                cell_node.append(cell_content);
                header_row_node.append(cell_node);
            }
            table_node.append(header_row_node);

            // Data rows
            for row in rows {
                let row_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                    NodeValue::TableRow(false), // Indicate data row
                    LineColumn { line: 0, column: 0 },
                ))));
                for cell in &row.cells {
                    let cell_node = arena.alloc(Node::new(RefCell::new(Ast::new(
                        NodeValue::TableCell,
                        LineColumn { line: 0, column: 0 },
                    ))));
                    let cell_content =
                        element_to_ast_node(arena, &cell.element, image_num, image_saver)?;
                    cell_node.append(cell_content);
                    row_node.append(cell_node);
                }
                table_node.append(row_node);
            }

            Ok(table_node)
        }

        _ => {
            let node = arena.alloc(Node::new(RefCell::new(Ast::new(
                NodeValue::Text("".to_string()),
                LineColumn { line: 0, column: 0 },
            ))));
            Ok(node)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::*;
    use crate::markdown::*;
    use crate::pdf;
    use crate::text;

    #[test]
    fn test() -> anyhow::Result<()> {
        let document = r#"
# First header

Paragraph  bla bla bla blabla bla bla blabla bla bla blabla bla bla blabla bla bla blabla bla bla blabla bla bla blabla bla bla blabla bla bla
blabla bla bla blabla bla bla blabla bla bla blabla bla bla bla

1. List item 1
2. List item 2
3. List item 3
   1. List item secode level 1
   2. List item secode level 2
4. List item 4
   1. List item secode level 3
   2. List item secode level 4
5. List item 5
   1. List item secode level 5

- List item one
- List item two
- List item three
- List item four
- List item five
    - List item zzz
- List item six
- List item seven

![Picture alt1](picture.png "Picture title1")

## Second header

| Syntax      | Description |
| ----------- | ----------- |
| Header      | Title       |
| Paragraph   | Text        |

Paragraph2  bla bla bla blabla bla bla blabla bla bla blabla bla bla blabla bla bla blabla bla bla blabla bla bla blabla bla bla blabla bla bla
blabla2 bla bla blabla bla bla blabla bla bla blabla bla bla bla"#;
        // println!("{:?}", document);
        let parsed = Transformer::parse_with_loader(&document.as_bytes().into(), disk_image_loader("test/data"));
        let document_string = std::str::from_utf8(document.as_bytes())?;
        println!("{}", document_string);
        assert!(parsed.is_ok());
        let parsed_document = parsed.unwrap();
        println!("==========================");
        println!("{:#?}", parsed_document);
        println!("==========================");
        let generated_result = Transformer::generate_with_saver(&parsed_document, disk_image_saver("test/data"));
        assert!(generated_result.is_ok());
        // println!("{:?}", generated_result.unwrap());
        let generated_bytes = generated_result?;
        let generated_text = std::str::from_utf8(&generated_bytes)?;
        println!("{}", generated_text);
        println!("==========================");
        let generated_result = text::Transformer::generate(&parsed_document);
        assert!(generated_result.is_ok());
        // println!("{:?}", generated_result.unwrap());
        let generated_bytes = generated_result?;
        let generated_text = std::str::from_utf8(&generated_bytes)?;
        println!("{}", generated_text);

        let generated_result = pdf::Transformer::generate(&parsed_document)?;
        std::fs::write("test/data/generated.pdf", generated_result)?;

        Ok(())
    }

    #[test]
    fn test_parse_header() {
        let document = r#"
# First header

## Second Header

### Third Header
            "#;

        let result_doc = Document {
            elements: vec![
                Header {
                    level: 1,
                    text: "First header".to_string(),
                },
                Header {
                    level: 2,
                    text: "Second Header".to_string(),
                },
                Header {
                    level: 3,
                    text: "Third Header".to_string(),
                },
            ],
            page_width: 210.0,
            page_height: 297.0,
            left_page_indent: 10.0,
            right_page_indent: 10.0,
            top_page_indent: 10.0,
            bottom_page_indent: 10.0,
            page_header: vec![],
            page_footer: vec![],
        };

        let parsed = Transformer::parse(&document.as_bytes().into()).unwrap();

        assert_eq!(parsed, result_doc)
    }


    #[test]
    fn test_parse_table() {
        let document = r#"
| Syntax      | Description |
| ----------- | ----------- |
| Header      | Title       |
| Paragraph   | Text        |
          "#;

        let result_doc = Document {
            elements: vec![Table {
                headers: vec![
                    TableHeader {
                        element: Text {
                            text: "Syntax".to_string(),
                            size: 14,
                        },
                        width: 30.0,
                    },
                    TableHeader {
                        element: Text {
                            text: "Description".to_string(),
                            size: 14,
                        },
                        width: 30.0,
                    },
                ],
                rows: vec![
                    TableRow {
                        cells: vec![
                            TableCell {
                                element: Text {
                                    text: "Header".to_string(),
                                    size: 14,
                                },
                            },
                            TableCell {
                                element: Text {
                                    text: "Title".to_string(),
                                    size: 14,
                                },
                            },
                        ],
                    },
                    TableRow {
                        cells: vec![
                            TableCell {
                                element: Text {
                                    text: "Paragraph".to_string(),
                                    size: 14,
                                },
                            },
                            TableCell {
                                element: Text {
                                    text: "Text".to_string(),
                                    size: 14,
                                },
                            },
                        ],
                    },
                ],
            }],
            page_width: 210.0,
            page_height: 297.0,
            left_page_indent: 10.0,
            right_page_indent: 10.0,
            top_page_indent: 10.0,
            bottom_page_indent: 10.0,
            page_header: vec![],
            page_footer: vec![],
        };

        let parsed = Transformer::parse_with_loader(&document.as_bytes().into(), disk_image_loader("test/data")).unwrap();

        assert_eq!(parsed, result_doc)
    }
}
