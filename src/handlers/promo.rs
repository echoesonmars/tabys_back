use crate::models::PromoCode;
use worker::*;

// 1. Проверка промокода (для корзины)
pub async fn check_promo(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("akniet_db")?;
    let body: serde_json::Value = req.json().await?;

    let code = body["code"].as_str().unwrap_or("").trim().to_uppercase();

    let stmt = d1
        .prepare("SELECT * FROM promocodes WHERE UPPER(TRIM(code)) = ? AND is_active = 1 LIMIT 1")
        .bind(&[code.into()])?;

    match stmt.first::<PromoCode>(None).await? {
        Some(promo) => Response::from_json(&promo),
        None => Response::error("Промокод не найден", 404),
    }
}

// 2. Список промокодов
pub async fn list_promos(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("akniet_db")?;
    let statement = d1.prepare("SELECT * FROM promocodes ORDER BY id DESC");
    let result = statement.all().await?;
    Response::from_json(&result.results::<PromoCode>()?)
}

// 3. Создание промокода
pub async fn create_promo(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("akniet_db")?;
    let promo: PromoCode = req.json().await?;

    d1.prepare("INSERT INTO promocodes (code, discount, is_active) VALUES (?, ?, 1)")
        .bind(&[promo.code.to_uppercase().into(), promo.discount.into()])?
        .run()
        .await?;

    Response::ok("Created")
}

// 4. Удаление промокода
pub async fn delete_promo(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let d1 = ctx.env.d1("akniet_db")?;
    let id = ctx.param("id").unwrap();

    d1.prepare("DELETE FROM promocodes WHERE id = ?")
        .bind(&[id.into()])?
        .run()
        .await?;

    Response::ok("Deleted")
}
