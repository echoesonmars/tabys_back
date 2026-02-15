mod handlers;
mod models;

use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let cors = Cors::default()
        .with_origins(vec!["*"])
        .with_methods(vec![Method::Get, Method::Post, Method::Options])
        .with_allowed_headers(vec!["Content-Type", "Authorization"])
        .with_max_age(3600);

    let router = Router::new();

    router
        .options("/api/cart-items", |_req, _ctx| {
            Response::empty()?.with_cors(&Cors::default().with_origins(vec!["*"]))
        })
        .options("/api/create-order", |_req, _ctx| {
            Response::empty()?.with_cors(&Cors::default().with_origins(vec!["*"]))
        })
        .options("/api/check-promo", |_req, _ctx| {
            Response::empty()?.with_cors(&Cors::default().with_origins(vec!["*"]))
        })
        .get("/", |_, _| Response::ok("Rust API OK"))
        .get_async("/api/categories", handlers::categories::list_categories)
        .post_async("/api/categories", handlers::categories::create_category)
        .get_async("/api/categories/:id", handlers::categories::get_category)
        .post_async(
            "/api/categories/edit/:id",
            handlers::categories::update_category,
        )
        .post_async(
            "/api/categories/delete",
            handlers::categories::delete_category,
        )
        .get_async("/api/products/:id", handlers::products::get_product)
        .post_async("/api/products/edit/:id", handlers::products::update_product)
        .get_async("/api/products", handlers::products::list_products)
        .post_async("/api/products", handlers::products::create_product)
        .post_async("/api/products/delete", handlers::products::delete_product)
        .get_async("/api/orders", handlers::orders::list_orders)
        .post_async("/api/orders/delete", handlers::orders::delete_order)
        .post_async("/api/cart-items", handlers::products::get_cart_items)
        .post_async("/api/create-order", handlers::orders::create_order) // Создать новый заказ
        .post_async("/api/check-promo", handlers::promo::check_promo)
        .get_async("/api/admin/promos", handlers::promo::list_promos)
        .post_async("/api/admin/promos", handlers::promo::create_promo)
        .delete_async("/api/admin/promos/:id", handlers::promo::delete_promo)
        .run(req, env)
        .await?
        .with_cors(&cors)
}
