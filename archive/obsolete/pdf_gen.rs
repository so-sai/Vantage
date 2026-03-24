use crate::{DocumentData, ExportConfig, ExportMode, VantageError};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::fs;
use std::process::Command;
use uuid::Uuid;

/// Converts DocumentData to a Typst source string and compiles it to PDF
pub fn render_to_pdf(doc: &DocumentData, config: &ExportConfig) -> Result<Vec<u8>, VantageError> {
    // 1. Convert Markdown to Typst markup
    let typst_content = internal_markdown_to_typst(&doc.text);

    // 2. Wrap in Template (Clean, Professional Theme)
    let full_source = apply_typst_template(&typst_content, doc, config);

    // 3. Compile using Typst CLI
    compile_typst(&full_source)
}

fn internal_markdown_to_typst(md: &str) -> String {
    let parser = Parser::new(md);
    let mut typst = String::new();
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    let equals = "=".repeat(level as usize);
                    typst.push_str(&format!("\n{} ", equals));
                }
                Tag::Paragraph => {
                    typst.push('\n');
                }
                Tag::List(Some(start)) => {
                    // Ordered list
                    typst.push_str(&format!("\n{}. ", start));
                }
                Tag::List(None) => {
                    // Unordered list
                    typst.push('\n');
                }
                Tag::Item => {
                    typst.push_str("- ");
                }
                Tag::BlockQuote => {
                    typst.push_str("\n#quote(block: true)[\n");
                }
                Tag::CodeBlock(_) => {
                    in_code_block = true;
                    typst.push_str("\n```\n");
                }
                Tag::Emphasis => typst.push('_'),
                Tag::Strong => typst.push('*'),
                Tag::Link { dest_url, .. } => {
                    typst.push_str(&format!("#link(\"{}\")[", dest_url));
                }
                Tag::Image { dest_url, .. } => {
                    // Basic image support
                    typst.push_str(&format!("#link(\"{}\")[IMAGE: ", dest_url));
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => typst.push('\n'),
                TagEnd::Paragraph => typst.push('\n'),
                TagEnd::BlockQuote => typst.push_str("]\n"),
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    typst.push_str("```\n");
                }
                TagEnd::Emphasis => typst.push('_'),
                TagEnd::Strong => typst.push('*'),
                TagEnd::Link => typst.push(']'),
                TagEnd::Image => typst.push(']'),
                _ => {}
            },
            Event::Text(text) => {
                let escaped = escape_typst_text(&text, in_code_block);
                typst.push_str(&escaped);
            }
            Event::Code(text) => {
                typst.push_str(&format!("`{}`", text));
            }
            Event::SoftBreak => typst.push(' '),
            Event::HardBreak => typst.push_str("\\\n"),
            Event::Rule => typst.push_str("\n#line(length: 100%, stroke: 0.5pt + gray)\n"),
            _ => {}
        }
    }
    typst
}

fn escape_typst_text(text: &str, in_code: bool) -> String {
    if in_code {
        return text.to_string();
    }
    // Simple escaping for common Typst special chars in prose
    text.replace("#", "\\#")
        .replace("$", "\\$")
        .replace("@", "\\@")
}

fn apply_typst_template(content: &str, doc: &DocumentData, config: &ExportConfig) -> String {
    let mode_str = match config.mode {
        ExportMode::Semantic => "Semantic (Internal)",
        ExportMode::Visual => "Visual Report",
        ExportMode::Audit => "AUDIT REPORT (Vantage Signed)",
    };

    let watermark = if config.mode == ExportMode::Audit {
        r#"
        place(
            center + horizon,
            rotate(24deg,
            text(80pt, fill: rgb("FF000020"))[CONFIDENTIAL])
        )
        "#
    } else {
        ""
    };

    let toc = if config.include_toc {
        r#"
        #outline(indent: auto)
        #pagebreak()
        "#
    } else {
        ""
    };

    format!(
        r#"
#set page(
    paper: "a4",
    margin: (x: 2.5cm, y: 2.5cm),
    numbering: "1 / 1",
)

// Font stack: Try common system fonts
#set text(
    font: ("New Computer Modern", "Times New Roman", "Arial", "Liberation Serif"),
    size: 11pt,
    lang: "en"
)

#set par(
    justify: true,
    leading: 0.65em,
)

#set heading(numbering: "1.1")

#show link: underline

// --- Header & Footer ---
#set page(
    header: [
        #set text(8pt, fill: gray)
        #smallcaps("Vantage Cognitive Engine") 
        #h(1fr)
        {mode}
    ],
    footer: [
        #set text(8pt, fill: gray)
        #h(1fr)
        Page #context counter(page).display()
    ]
)

{watermark}

// --- Title ---
#align(center)[
    #text(18pt, weight: "bold")[{title}]
    \
    #text(10pt, style: "italic")[Source: {source}]
]

#line(length: 100%, stroke: 0.5pt + gray)

#v(1cm)

{toc}

// --- Content ---
{content}
"#,
        mode = mode_str,
        watermark = watermark,
        title = doc
            .headers
            .first()
            .map(|s| s.as_str())
            .unwrap_or("Vantage Document"),
        source = doc.metadata.source_type,
        toc = toc,
        content = content
    )
}

fn compile_typst(source: &str) -> Result<Vec<u8>, VantageError> {
    let id = Uuid::new_v4();
    let temp_dir = std::env::temp_dir();
    let input_path = temp_dir.join(format!("vantage_{}.typ", id));
    let output_path = temp_dir.join(format!("vantage_{}.pdf", id));

    // Write source file
    fs::write(&input_path, source)
        .map_err(|e| VantageError::ExportError(format!("Failed to write typst source: {}", e)))?;

    // Run Typst CLI
    let output = Command::new("typst")
        .arg("compile")
        .arg(&input_path)
        .arg(&output_path)
        .output()
        .map_err(|e| {
            VantageError::ExportError(format!(
                "Failed to execute typst: {}. Is 'typst' installed?",
                e
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(VantageError::ExportError(format!(
            "Typst compilation failed: {}",
            stderr
        )));
    }

    // Read PDF output
    let pdf_bytes = fs::read(&output_path)
        .map_err(|e| VantageError::ExportError(format!("Failed to read generated PDF: {}", e)))?;

    // Cleanup (best effort)
    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(output_path);

    Ok(pdf_bytes)
}
