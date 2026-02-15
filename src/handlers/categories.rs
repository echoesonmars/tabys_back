use crate::models::Category;
use uuid::Uuid;
use worker::*;

pub async fn list_categories(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Подключение к базе данных
    let d1 = ctx.env.d1("tabys_db")?;

    let statement = d1.prepare("SELECT * FROM categories");

    match statement.all().await {
        Ok(result) => {
            let categories = result.results::<Category>()?;
            Response::from_json(&categories)
        }
        Err(e) => Response::error(format!("D1 Error: {}", e), 500),
    }
}

//Добавление категории
pub async fn create_category(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let form = req.form_data().await?;
    let bucket = ctx.env.bucket("tabys_bucket")?;
    let d1 = ctx.env.d1("tabys_db")?;

    let name = form
        .get("name")
        .and_then(|e| {
            if let FormEntry::Field(s) = e {
                Some(s)
            } else {
                None
            }
        })
        .unwrap_or_default();
    let name_kk = form
        .get("name_kk")
        .and_then(|e| {
            if let FormEntry::Field(s) = e {
                Some(s)
            } else {
                None
            }
        })
        .unwrap_or_default();
    let slug = form
        .get("slug")
        .and_then(|e| {
            if let FormEntry::Field(s) = e {
                Some(s)
            } else {
                None
            }
        })
        .unwrap_or_default()
        .to_lowercase();

    let parent_id = form
        .get("parent_id")
        .and_then(|e| {
            if let FormEntry::Field(s) = e {
                Some(s)
            } else {
                None
            }
        })
        .and_then(|s| s.parse::<i32>().ok());

    let mut image_url = String::new();

    // Обработка картинки категории
    if let Some(entry) = form.get("imageFile") {
        if let FormEntry::File(file) = entry {
            let file_name = format!("cat-{}.jpg", Uuid::new_v4());
            let bytes = file.bytes().await?;
            bucket.put(&file_name, bytes).execute().await?;
            image_url = format!("https://img.tabys-go.ru/{}", file_name);
        }
    }

    // Сохраняем в базу
    let result = d1
        .prepare(
            "INSERT INTO categories (name, name_kk, slug, parent_id, image) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&[
            name.trim().into(),
            name_kk.trim().into(),
            slug.trim().into(),
            // Магия тут: если parent_id есть - берем его, если нет - явно шлем NULL
            parent_id
                .map(|id| id.into())
                .unwrap_or(wasm_bindgen::JsValue::NULL),
            image_url.into(),
        ])?
        .run()
        .await;

    match result {
        Ok(_) => Response::ok("Category created"),
        Err(e) => {
            if e.to_string().contains("UNIQUE constraint failed") {
                Response::error("Slug уже существует", 400)
            } else {
                Response::error(format!("D1 Error: {}", e), 500)
            }
        }
    }
}

//Удаление категории

pub async fn delete_category(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("tabys_db")?;
    let bucket = ctx.env.bucket("tabys_bucket")?;

    let body: serde_json::Value = req.json().await?;
    let id = body["id"]
        .as_i64()
        .or_else(|| body["id"].as_str().and_then(|s| s.parse::<i64>().ok()))
        .unwrap_or(0) as i32;

    let image_url = body["image"].as_str().unwrap_or("");

    if id == 0 {
        return Response::error("ID категории не передан", 400);
    }

    // 1. Удаляем из базы
    d1.prepare("DELETE FROM categories WHERE id = ?")
        .bind(&[id.into()])?
        .run()
        .await?;

    // 2. Удаляем картинку из базы картинок, если она есть
    if !image_url.is_empty() && image_url.contains('/') {
        if let Some(file_name) = image_url.split('/').last() {
            let _ = bucket.delete(file_name).await;
        }
    }

    Response::ok("Category deleted")
}

//Получить категорию
pub async fn get_category(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let id = ctx.param("id").map(|s| s.to_string()).unwrap_or_default();
    let d1 = ctx.env.d1("tabys_db")?;

    let statement = d1
        .prepare("SELECT * FROM categories WHERE id = ?")
        .bind(&[id.into()])?;

    match statement.first::<Category>(None).await {
        Ok(Some(cat)) => Response::from_json(&cat),
        Ok(None) => Response::error("Категория не найдена", 404),
        Err(e) => Response::error(format!("D1 Error: {}", e), 500),
    }
}

// 2. Обновить категорию
pub async fn update_category(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let id = ctx.param("id").map(|s| s.to_string()).unwrap_or_default();
    let form = req.form_data().await?;
    let d1 = ctx.env.d1("tabys_db")?;
    let bucket = ctx.env.bucket("tabys_bucket")?;

    // Извлекаем поля с защитой от пустых значений
    let name = form
        .get("name")
        .and_then(|e| {
            if let FormEntry::Field(s) = e {
                Some(s)
            } else {
                None
            }
        })
        .unwrap_or_default();
    let name_kk = form
        .get("name_kk")
        .and_then(|e| {
            if let FormEntry::Field(s) = e {
                Some(s)
            } else {
                None
            }
        })
        .unwrap_or_default();
    let slug = form
        .get("slug")
        .and_then(|e| {
            if let FormEntry::Field(s) = e {
                Some(s)
            } else {
                None
            }
        })
        .unwrap_or_default()
        .to_lowercase();

    // Обработка parent_id: если пустая строка, то  NULL
    let parent_id = form
        .get("parent_id")
        .and_then(|e| {
            if let FormEntry::Field(s) = e {
                Some(s)
            } else {
                None
            }
        })
        .and_then(|s| {
            if s.is_empty() {
                None
            } else {
                s.parse::<i32>().ok()
            }
        });

    // Текущая картинка
    let mut image_url = form
        .get("current_image")
        .and_then(|e| {
            if let FormEntry::Field(s) = e {
                Some(s)
            } else {
                None
            }
        })
        .unwrap_or_default();

    // Если загрузили новый файл
    if let Some(entry) = form.get("imageFile") {
        if let FormEntry::File(file) = entry {
            if file.size() > 0 {
                let file_name = format!("cat-{}.jpg", uuid::Uuid::new_v4());
                bucket
                    .put(&file_name, file.bytes().await?)
                    .execute()
                    .await?;
                image_url = format!("https://img.tabys-go.ru/{}", file_name);
            }
        }
    }

    // UPDATE в базе
    let result = d1
        .prepare("UPDATE categories SET name=?, name_kk=?, slug=?, parent_id=?, image=? WHERE id=?")
        .bind(&[
            name.trim().into(),
            name_kk.trim().into(),
            slug.trim().into(),
            parent_id
                .map(|id| id.into())
                .unwrap_or(wasm_bindgen::JsValue::NULL), // Явный NULL
            image_url.into(),
            id.into(),
        ])?
        .run()
        .await;

    match result {
        Ok(_) => Response::ok("Updated"),
        Err(e) => Response::error(format!("D1 Update Error: {}", e), 500),
    }
}
