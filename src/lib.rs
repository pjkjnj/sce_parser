use wasm_bindgen::prelude::*;
use scraper::{Html, Selector};
use serde::Serialize;

#[derive(Serialize)]
pub struct CardInfo {
    name: String,
    image: String,
    credits: Option<String>,
}

#[derive(Serialize)]
pub struct GameInfo {
    title: Option<String>,
    total_credits: Option<String>,
    cards: Vec<CardInfo>,
}

#[wasm_bindgen]
pub fn parse_sce_html(html: &str) -> String {
    let doc = Html::parse_document(html);

    // selectors
    let title_sel = Selector::parse("div.inventory_gameinfo h2, h1").unwrap();
    let card_sel = Selector::parse("div.inventory_gamecards div.inventory_gamecard").unwrap();
    let img_sel = Selector::parse("img").unwrap();
    let name_sel = Selector::parse(".inventory_gamecard_name").unwrap();
    let credits_sel = Selector::parse(".credit_value").unwrap();

    // title and total credits
    let title = doc
        .select(&title_sel)
        .next()
        .map(|t| t.text().collect::<String>().trim().to_string());

    let total_credits = doc
        .select(&credits_sel)
        .next()
        .map(|v| v.text().collect::<String>().trim().to_string());

    // cards
    let mut cards = Vec::new();
    for card in doc.select(&card_sel) {
        let name = card
            .select(&name_sel)
            .next()
            .map(|n| n.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let image = card
            .select(&img_sel)
            .next()
            .and_then(|i| i.value().attr("src"))
            .unwrap_or("")
            .to_string();

        let credits = card
            .select(&credits_sel)
            .next()
            .map(|v| v.text().collect::<String>().trim().to_string());

        cards.push(CardInfo { name, image, credits });
    }

    let info = GameInfo {
        title,
        total_credits,
        cards,
    };

    serde_json::to_string(&info).unwrap_or_else(|_| "{}".to_string())
  }
             
