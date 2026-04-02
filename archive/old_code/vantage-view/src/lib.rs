use vantage_core::Sheet;

pub mod spatial_hud;
pub use spatial_hud::generate_jet_fighter_hud_schema;

#[derive(Debug)]
pub struct CodeModule {
    pub file_path: String,
    pub content: String,
}

#[derive(Debug)]
pub struct MarkdownSpec {
    pub file_path: String,
    pub content: String,
}

#[derive(Debug)]
pub struct SpatialOverlay {
    pub excel_data: Vec<Sheet>,
    pub markdown_specs: Vec<MarkdownSpec>,
    pub code_modules: Vec<CodeModule>,
}

impl SpatialOverlay {
    pub fn new() -> Self {
        SpatialOverlay {
            excel_data: Vec::new(),
            markdown_specs: Vec::new(),
            code_modules: Vec::new(),
        }
    }

    pub fn add_excel(&mut self, data: Vec<Sheet>) {
        self.excel_data.extend(data);
    }

    pub fn add_markdown(&mut self, spec: MarkdownSpec) {
        self.markdown_specs.push(spec);
    }

    pub fn add_code(&mut self, module: CodeModule) {
        self.code_modules.push(module);
    }

    // Placeholder for rendering logic
    pub fn render(&self) -> String {
        format!(
            "Spatial Overlay:\nExcel Sheets: {}\nMarkdown Specs: {}\nCode Modules: {}",
            self.excel_data.len(),
            self.markdown_specs.len(),
            self.code_modules.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_overlay() {
        let mut overlay = SpatialOverlay::new();
        overlay.add_excel(vec![Sheet {
            name: "Test".to_string(),
            data: vec![],
        }]);
        assert_eq!(overlay.excel_data.len(), 1);
    }
}
