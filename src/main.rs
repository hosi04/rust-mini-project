use anyhow::Result;
use clickhouse::{Client, Row};
use dotenvy::dotenv;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Deserialize)]
struct ApiResponse {
    data: ApiData,
}

#[derive(Debug, Deserialize)]
struct ApiData {
    meta_data: Metadata,
    products: Vec<ProductList>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    products_total: u32,
}

#[derive(Debug, Deserialize)]
struct ProductList {
    id: u64,
    sku: Option<String>,
    name: Option<String>,
    english_name: Option<String>,
    category_name: Option<String>,
    category_id: u32,
    brand: Brand,
    market_price: Option<u64>,
    price: Option<u64>,
    discount_percent: u32,
    quantity: Option<u64>,
    bought_count: Option<u64>,
    rating: Option<Rating>,
    promotion_text: Option<String>,
    is_spa: bool,
}

#[derive(Debug, Deserialize)]
struct Brand {
    id: u32,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Rating {
    avg_rate: f64,
    total_rate: u32,
}

#[derive(Debug, Serialize, Deserialize, Row)]
struct ProductRow {
    id: u64,
    sku: String,
    name: String,
    english_name: String,
    category_name: String,
    category_id: u32,
    brand_id: u32,
    brand_name: String,
    market_price: u64,
    price: u64,
    discount_percent: u32,
    quantity: u64,
    bought_count: u64,
    avg_rate: f64,
    total_rate: u32,
    promotion_text: String,
    is_spa: u8,
}

impl From<ProductList> for ProductRow {
    fn from(p: ProductList) -> Self {
        Self {
            id: p.id,
            sku: p.sku.unwrap_or_default(),
            name: p.name.unwrap_or_default(),
            english_name: p.english_name.unwrap_or_default(),
            category_name: p.category_name.unwrap_or_default(),
            category_id: p.category_id,
            brand_id: p.brand.id,
            brand_name: p.brand.name.unwrap_or_default(),
            market_price: p.market_price.unwrap_or(0),
            price: p.price.unwrap_or(0),
            discount_percent: p.discount_percent,
            quantity: p.quantity.unwrap_or(0),
            bought_count: p.bought_count.unwrap_or(0),
            avg_rate: p.rating.as_ref().map(|r| r.avg_rate).unwrap_or(0.0),
            total_rate: p.rating.as_ref().map(|r| r.total_rate).unwrap_or(0),
            promotion_text: p.promotion_text.unwrap_or_default(),
            is_spa: p.is_spa as u8,
        }
    }
}

const PAGE_SIZE: u32 = 100;
const BASE_URL: &str = "https://api.hasaki.vn/mobile/v1/main/search";
const FORM_KEY: &str = "96bc5a24414b36ff5c3f00666f681f03";

async fn fetch_page(http: &HttpClient, page: u32) -> Result<ApiResponse> {
    let url = format!(
        "{}?page={}&size={}&has_meta_data=1&form_key={}",
        BASE_URL, page, PAGE_SIZE, FORM_KEY
    );

    let resp = http
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await?
        .json::<ApiResponse>()
        .await?;

    Ok(resp)
}

async fn fetch_all(http: &HttpClient) -> Result<Vec<ProductList>> {
    let first = fetch_page(http, 1).await?;
    let total = first.data.meta_data.products_total;
    let total_pages = (total + PAGE_SIZE - 1) / PAGE_SIZE;

    println!(">>> Total products: {} | Pages: {}", total, total_pages);

    let mut all = first.data.products;

    for page in 2..=total_pages {
        println!(">>> Fetching page {}/{}...", page, total_pages);
        let resp = fetch_page(http, page).await?;
        all.extend(resp.data.products);
    }
    Ok(all)
}

async fn insert_products(ch: &Client, products: Vec<ProductList>) -> Result<()> {
    let total = products.len();
    let mut insert = ch.insert::<ProductRow>("products").await?;

    for p in products {
        insert.write(&ProductRow::from(p)).await?;
    }

    insert.end().await?;
    println!(">>> Inserted {} rows", total);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let http = HttpClient::new();

    let ch_host = env::var("CLICKHOUSE_HOST").expect("error!");
    let ch_port = env::var("CLICKHOUSE_PORT").expect("error!");
    let ch_user = env::var("CLICKHOUSE_USER").expect("error!");
    let ch_password = env::var("CLICKHOUSE_PASSWORD").expect("error!");
    let ch_database = env::var("CLICKHOUSE_DATABASE").expect("error connection!");

    let ch = Client::default()
        .with_url(format!("http://{}:{}", ch_host, ch_port))
        .with_user(ch_user)
        .with_password(ch_password)
        .with_database(ch_database);

    let products = fetch_all(&http).await?;
    insert_products(&ch, products).await?;

    Ok(())
}
