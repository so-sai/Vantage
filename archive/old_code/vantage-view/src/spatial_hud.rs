use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32, // Abstraction level: 0 = code, 1 = docs, etc.
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HudElement {
    pub id: String,
    pub element_type: String, // "card", "gauge", "alert", "overlay"
    pub position: Position,
    pub content: serde_json::Value,
    pub style: HudStyle,
    pub trigger: Option<String>, // e.g., "hover_function", "high_cpu"
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HudStyle {
    pub transparency: f32,
    pub color: String,
    pub size: (f32, f32), // width, height
    pub border: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpatialHud {
    pub overlay_type: String, // "transparent_overlay"
    pub transparency: f32,
    pub elements: Vec<HudElement>,
}

impl SpatialHud {
    pub fn new() -> Self {
        SpatialHud {
            overlay_type: "transparent_overlay".to_string(),
            transparency: 0.85,
            elements: Vec::new(),
        }
    }

    pub fn add_element(&mut self, element: HudElement) {
        self.elements.push(element);
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

pub fn generate_jet_fighter_hud_schema() -> SpatialHud {
    let mut hud = SpatialHud::new();

    // Altitude Gauge (Abstraction Level)
    hud.add_element(HudElement {
        id: "abstraction_gauge".to_string(),
        element_type: "gauge".to_string(),
        position: Position {
            x: 10.0,
            y: 10.0,
            z: 0.0,
        },
        content: serde_json::json!({
            "label": "Abstraction Level",
            "value": 0.5,
            "unit": "layers",
            "range": [0.0, 1.0]
        }),
        style: HudStyle {
            transparency: 0.9,
            color: "#00FF00".to_string(),
            size: (150.0, 50.0),
            border: Some("solid 1px #00FF00".to_string()),
        },
        trigger: None,
    });

    // Processing Speed Indicator
    hud.add_element(HudElement {
        id: "processing_speed".to_string(),
        element_type: "gauge".to_string(),
        position: Position {
            x: 170.0,
            y: 10.0,
            z: 0.0,
        },
        content: serde_json::json!({
            "label": "Processing Speed",
            "value": 95.0,
            "unit": "ops/sec",
            "status": "nominal"
        }),
        style: HudStyle {
            transparency: 0.9,
            color: "#FFFF00".to_string(),
            size: (150.0, 50.0),
            border: None,
        },
        trigger: None,
    });

    // Vibe Card (appears on hover)
    hud.add_element(HudElement {
        id: "vibe_card".to_string(),
        element_type: "card".to_string(),
        position: Position {
            x: 0.0,
            y: 0.0,
            z: 0.5,
        }, // Relative to cursor
        content: serde_json::json!({
            "title": "Function Vibe",
            "description": "Pulled from RESEARCH_CANVAS.md",
            "related_excel_rows": [],
            "drift_status": "synced"
        }),
        style: HudStyle {
            transparency: 0.95,
            color: "#FFFFFF".to_string(),
            size: (300.0, 200.0),
            border: Some("solid 2px #00FFFF".to_string()),
        },
        trigger: Some("hover_function".to_string()),
    });

    // Resource Alert
    hud.add_element(HudElement {
        id: "resource_alert".to_string(),
        element_type: "alert".to_string(),
        position: Position {
            x: 330.0,
            y: 10.0,
            z: 0.0,
        },
        content: serde_json::json!({
            "message": "High CPU detected. Switching to Lean Mode.",
            "level": "warning"
        }),
        style: HudStyle {
            transparency: 0.8,
            color: "#FF0000".to_string(),
            size: (200.0, 30.0),
            border: Some("solid 1px #FF0000".to_string()),
        },
        trigger: Some("high_cpu".to_string()),
    });

    // Data Targets (Excel/Word markers)
    hud.add_element(HudElement {
        id: "data_targets".to_string(),
        element_type: "overlay".to_string(),
        position: Position {
            x: 0.0,
            y: 0.0,
            z: 0.8,
        },
        content: serde_json::json!({
            "targets": [
                {"file": "data.xlsx", "sheet": "Sheet1", "rows": [1, 2, 3]},
                {"file": "spec.docx", "paragraphs": [5, 10]}
            ]
        }),
        style: HudStyle {
            transparency: 0.7,
            color: "#0000FF".to_string(),
            size: (0.0, 0.0), // Auto-size
            border: None,
        },
        trigger: None,
    });

    hud
}
