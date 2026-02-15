use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub name_kk: Option<String>,
    pub price: f64,
    pub old_price: Option<f64>,
    pub image: Option<String>,
    pub category: Option<String>,
    pub category_id: Option<i32>,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub description_kk: Option<String>,
    pub stock: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Category {
    pub id: i32,
    pub parent_id: Option<i32>,
    pub name: String,
    pub name_kk: String,
    pub image: Option<String>,
    pub slug: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Order {
    pub id: i32,
    pub customer_name: String,
    pub customer_phone: String,
    pub address: String,
    pub comment: Option<String>,
    pub items_json: String,
    pub total_price: f64,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromoCode {
    pub id: Option<i32>,
    pub code: String,
    pub discount: i32,
    pub is_active: Option<i32>,
}
