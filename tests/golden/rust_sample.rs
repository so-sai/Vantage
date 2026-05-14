// @epistemic:f1
pub fn calculate_total(items: Vec<f64>) -> f64 {
    items.iter().sum()
}

// @epistemic:s1
pub struct Invoice {
    pub id: String,
    pub amount: f64,
}

// @epistemic:c1
pub const VERSION: &str = "1.2.5";
