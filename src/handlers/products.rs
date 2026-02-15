use crate::models::Product;
use uuid::Uuid;
use worker::*;

// 1. Получение списка
pub async fn list_products(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("akniet_db")?;
    let url = req.url()?;
    let query_pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();

    // Проверяем, пришел ли флаг admin=true
    let is_admin = query_pairs.iter().any(|(k, v)| k == "admin" && v == "true");

    // Если админ — показываем всё, если покупатель — только stock > 0
    let mut sql = if is_admin {
        "SELECT * FROM products WHERE 1=1".to_string()
    } else {
        "SELECT * FROM products WHERE stock > 0".to_string()
    };

    let mut params: Vec<wasm_bindgen::JsValue> = Vec::new();

    for (key, value) in &query_pairs {
        match key.as_str() {
            "categoryId" | "category_id" => {
                if let Ok(id) = value.parse::<i32>() {
                    sql.push_str(" AND category_id = ?");
                    params.push(id.into());
                }
            }
            "q" if !value.is_empty() => {
                sql.push_str(" AND (name LIKE ? OR name_kk LIKE ? OR description LIKE ?)");
                let pattern = format!("%{}%", value);
                params.push(pattern.clone().into());
                params.push(pattern.clone().into());
                params.push(pattern.into());
            }
            _ => {}
        }
    }

    sql.push_str(" ORDER BY id DESC LIMIT 100");

    let statement = d1.prepare(&sql).bind(&params)?;
    let result = statement.all().await?;
    Response::from_json(&result.results::<Product>()?)
}

// 3. Создание товара
pub async fn create_product(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let form = req.form_data().await?; // Используем встроенный метод
    let bucket = ctx.env.bucket("akniet_bucket")?;
    let d1 = ctx.env.d1("akniet_db")?;

    let mut name = String::new();
    let mut name_kk = String::new();
    let mut category_id: i32 = 0;
    let mut price: f64 = 0.0;
    let mut old_price: Option<f64> = None;
    let mut unit = String::new();
    let mut stock: i32 = 0;
    let mut description = String::new();
    let mut description_kk = String::new();
    let mut image_urls: Vec<String> = Vec::new();

    //получаем значения по именам напрямую
    if let Some(FormEntry::Field(val)) = form.get("name") {
        name = val;
    }
    if let Some(FormEntry::Field(val)) = form.get("name_kk") {
        name_kk = val;
    }
    if let Some(FormEntry::Field(val)) = form.get("category_id") {
        category_id = val.parse().unwrap_or(0);
    }
    if let Some(FormEntry::Field(val)) = form.get("price") {
        price = val.parse().unwrap_or(0.0);
    }

    if let Some(FormEntry::Field(val)) = form.get("old_price") {
        if !val.is_empty() {
            old_price = val.parse().ok(); // Если не пусто, парсим в f64
        }
    }
    if let Some(FormEntry::Field(val)) = form.get("unit") {
        unit = val;
    }
    if let Some(FormEntry::Field(val)) = form.get("stock") {
        stock = val.parse().unwrap_or(0);
    }
    if let Some(FormEntry::Field(val)) = form.get("description") {
        description = val;
    }
    if let Some(FormEntry::Field(val)) = form.get("description_kk") {
        description_kk = val;
    }

    // Обработка файлов
    let files = form.get_all("imageFiles").unwrap_or_default();
    for entry in files {
        if let FormEntry::File(file) = entry {
            let file_name = format!("prod-{}.jpg", Uuid::new_v4());
            let bytes = file.bytes().await?;
            bucket.put(&file_name, bytes).execute().await?;
            image_urls.push(format!("https://img.tabys-go.ru/{}", file_name));
        }
    }

    let images_json = serde_json::to_string(&image_urls).unwrap_or_default();

    let query = "INSERT INTO products (name, name_kk, category_id, price, old_price, unit, image, description, description_kk, stock) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
    d1.prepare(query)
        .bind(&[
            name.into(),
            name_kk.into(),
            category_id.into(),
            price.into(),
            old_price.into(),
            unit.into(),
            images_json.into(),
            description.into(),
            description_kk.into(),
            stock.into(),
        ])?
        .run()
        .await?;

    Response::ok("Success")
}

// удаление товара

pub async fn delete_product(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("akniet_db")?;
    let bucket = ctx.env.bucket("akniet_bucket")?;

    let body: serde_json::Value = req.json().await?;

    // Получаем ID и конвертирем для базы
    let id = body["id"]
        .as_i64()
        .or_else(|| body["id"].as_str().and_then(|s| s.parse::<i64>().ok()))
        .unwrap_or(0) as i32;

    let image_url = body["image"].as_str().unwrap_or("");

    if id == 0 {
        return Response::error("ID товара не передан или равен 0", 400);
    }

    // Удаляем из базы
    d1.prepare("DELETE FROM products WHERE id = ?")
        .bind(&[id.into()])?
        .run()
        .await?;

    if !image_url.is_empty() && image_url.contains('/') {
        if let Some(file_name) = image_url.split('/').last() {
            let _ = bucket.delete(file_name).await;
        }
    }

    Response::ok("Deleted")
}

// поиск одного товара
pub async fn get_product(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let id = ctx.param("id").map(|s| s.to_string()).unwrap_or_default();
    let d1 = ctx.env.d1("akniet_db")?;

    let statement = d1
        .prepare("SELECT * FROM products WHERE id = ?")
        .bind(&[id.into()])?;

    match statement.first::<Product>(None).await {
        Ok(Some(product)) => Response::from_json(&product),
        Ok(None) => Response::error("Товар не найден", 404),
        Err(e) => Response::error(format!("Ошибка базы данных: {}", e), 500),
    }
}

// изменение продукта

pub async fn update_product(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let id = ctx.param("id").map(|s| s.to_string()).unwrap_or_default();
    let form = req.form_data().await?;
    let d1 = ctx.env.d1("akniet_db")?;
    let bucket = ctx.env.bucket("akniet_bucket")?;

    let name = match form.get("name") {
        Some(FormEntry::Field(s)) if s != "undefined" => s,
        _ => String::new(),
    };
    let name_kk = match form.get("name_kk") {
        Some(FormEntry::Field(s)) if s != "undefined" => s,
        _ => String::new(),
    };
    let category_id = match form.get("category_id") {
        Some(FormEntry::Field(s)) if s != "undefined" => s.parse::<i32>().unwrap_or(0),
        _ => 0,
    };
    let price = match form.get("price") {
        Some(FormEntry::Field(s)) if s != "undefined" => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    };

    let old_price = match form.get("old_price") {
        Some(FormEntry::Field(s)) => {
            if s.is_empty() || s == "undefined" || s == "null" {
                wasm_bindgen::JsValue::NULL // Явно шлем NULL в базу
            } else {
                match s.parse::<f64>() {
                    Ok(val) => wasm_bindgen::JsValue::from_f64(val),
                    Err(_) => wasm_bindgen::JsValue::NULL,
                }
            }
        }
        _ => wasm_bindgen::JsValue::NULL,
    };

    let unit = match form.get("unit") {
        Some(FormEntry::Field(s)) if s != "undefined" => s,
        _ => String::new(),
    };

    let stock = match form.get("stock") {
        Some(FormEntry::Field(s)) if s != "undefined" => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    };

    let description = match form.get("description") {
        Some(FormEntry::Field(s)) if s != "undefined" => s,
        _ => String::new(),
    };
    let description_kk = match form.get("description_kk") {
        Some(FormEntry::Field(s)) if s != "undefined" => s,
        _ => String::new(),
    };

    let mut final_images: Vec<String> = match form.get("remainingImages") {
        Some(FormEntry::Field(s)) if s != "undefined" && !s.is_empty() => {
            serde_json::from_str(&s).unwrap_or_default()
        }
        _ => Vec::new(),
    };

    if let Some(entries) = form.get_all("imageFiles") {
        for entry in entries {
            if let FormEntry::File(file) = entry {
                let file_name = format!("prod-{}.jpg", Uuid::new_v4());
                let bytes = file.bytes().await?;
                bucket.put(&file_name, bytes).execute().await?;
                final_images.push(format!("https://img.tabys-go.ru/{}", file_name));
            }
        }
    }
    let images_json = serde_json::to_string(&final_images).unwrap_or_default();

    let query = "UPDATE products SET name=?1, name_kk=?2, category_id=?3, price=?4, old_price=?5, unit=?6, image=?7, description=?8, description_kk=?9, stock=?10 WHERE id=?11";

    d1.prepare(query)
        .bind(&[
            name.into(),
            name_kk.into(),
            category_id.into(),
            price.into(),
            old_price,
            unit.into(),
            images_json.into(),
            description.into(),
            description_kk.into(),
            stock.into(),
            id.into(),
        ])?
        .run()
        .await?;

    Response::ok("Updated")
}

//получение товаров для корзины
pub async fn get_cart_items(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("akniet_db")?;
    let body: serde_json::Value = req.json().await?;

    let ids_raw = body["ids"].as_array().ok_or("No IDs")?;

    let id_list: Vec<i32> = ids_raw
        .iter()
        .filter_map(|v| {
            v.as_str()
                .and_then(|s| s.parse::<i32>().ok()) // если строка
                .or_else(|| v.as_i64().map(|n| n as i32)) // если число
        })
        .collect();

    if id_list.is_empty() {
        return Response::from_json(&Vec::<Product>::new());
    }

    let placeholders = id_list.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!("SELECT * FROM products WHERE id IN ({})", placeholders);

    let params: Vec<wasm_bindgen::JsValue> = id_list.into_iter().map(|n| n.into()).collect();

    let statement = d1.prepare(&query).bind(&params)?;
    let result = statement.all().await?;

    Response::from_json(&result.results::<Product>()?)
}
