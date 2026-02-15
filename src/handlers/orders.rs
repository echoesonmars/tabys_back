use crate::models::Order;
use worker::*;

// Список заказов
pub async fn list_orders(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("tabys_db")?;
    let statement = d1.prepare("SELECT * FROM orders ORDER BY created_at DESC");

    match statement.all().await {
        Ok(result) => Response::from_json(&result.results::<Order>()?),
        Err(e) => Response::error(format!("D1 Error: {}", e), 500),
    }
}

// Удаление заказа
pub async fn delete_order(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("tabys_db")?;
    let body: serde_json::Value = req.json().await?;

    let id = body["id"]
        .as_i64()
        .or_else(|| body["id"].as_str().and_then(|s| s.parse::<i64>().ok()))
        .unwrap_or(0) as i32;

    if id == 0 {
        return Response::error("ID заказа не указан", 400);
    }

    d1.prepare("DELETE FROM orders WHERE id = ?")
        .bind(&[id.into()])?
        .run()
        .await?;
    Response::ok("Order deleted")
}

//создание заказа
pub async fn create_order(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("tabys_db")?;
    let body: serde_json::Value = req.json().await?;

    let customer = &body["customer"];
    let items = body["items"].as_array().ok_or("No items")?;

    let order_query = d1.prepare(
        "INSERT INTO orders (customer_name, customer_phone, address, comment, items_json, total_price, status, created_at) 
         VALUES (?, ?, ?, ?, ?, ?, 'new', datetime('now'))"
    ).bind(&[
        customer["name"].as_str().unwrap_or("").into(),
        customer["phone"].as_str().unwrap_or("").into(),
        customer["address"].as_str().unwrap_or("").into(),
        customer["comment"].as_str().unwrap_or("").into(),
        body["items"].to_string().into(),
        body["total"].as_f64().unwrap_or(0.0).into(),
    ])?;

    //Запросы на списание остатков
    let mut queries = vec![order_query];
    for item in items {
        let id = item["id"].as_str().unwrap_or("0");
        let qty = item["quantity"].as_f64().unwrap_or(0.0);

        let stock_query = d1
            .prepare("UPDATE products SET stock = stock - ? WHERE id = ? AND stock >= ?")
            .bind(&[qty.into(), id.into(), qty.into()])?;

        queries.push(stock_query);
    }

    d1.batch(queries).await?;

    Response::from_json(&serde_json::json!({ "success": true }))
}
