use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use web_sys::{Request, RequestInit, RequestMode, Response, Window};
use js_sys::Promise;
use serde::Serialize;
use scraper::{Html, Selector};

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

// helper: perform fetch and return response text (async)
async fn fetch_text(url: &str) -> Result<String, JsValue> {
    let window: Window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(url, &opts)
        .map_err(|e| JsValue::from(e.as_string().unwrap_or_else(|| "request error".into())))?;

    // Some sites block cross-origin, but GM_xmlhttpRequest (Tampermonkey) is different.
    // In browser, this fetch will work for pages that allow CORS. Tampermonkey will call wasm function
    // with HTML passed in (we also provide fetch-based API for pages that allow it).
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into().map_err(|_| JsValue::from_str("not a response"))?;
    if !resp.ok() {
        return Err(JsValue::from_str(&format!("HTTP error: {}", resp.status())));
    }
    let text_promise = resp.text().map_err(|_| JsValue::from_str("failed to get text"))?;
    let txt = JsFuture::from(text_promise).await?;
    Ok(txt.as_string().unwrap_or_default())
}

use wasm_bindgen_futures::JsFuture;

// Exported function: fetch SCE page for given appid and parse
#[wasm_bindgen]
pub fn fetch_and_parse(appid: &str) -> Promise {
    let url = format!("https://www.steamcardexchange.net/index.php?inventorygame-appid-{}", appid);

    // convert the Rust future into JS Promise
    future_to_promise(async move {
        // 1) fetch HTML (use browser fetch)
        let html = match fetch_text(&url).await {
            Ok(s) => s,
            Err(e) => {
                // Return JSON with error field
                let err = serde_json::json!({ "error": format!("fetch failed: {:?}", e) });
                return Ok(JsValue::from_str(&err.to_string()));
            }
        };

        // 2) parse HTML with scraper
        let doc = Html::parse_document(&html);

        // selectors (best-effort; SCE pages vary)
        let title_sel = Selector::parse("div.inventory_gameinfo h2, div.inventory_gameinfo h1, h1").unwrap();
        let card_sel = Selector::parse("div.inventory_gamecards div.inventory_gamecard").unwrap();
        let img_sel = Selector::parse("img").unwrap();
        let name_sel = Selector::parse(".inventory_gamecard_name").unwrap();
        let credits_sel = Selector::parse(".credit_value").unwrap();

        let title = doc.select(&title_sel).next().map(|t| t.text().collect::<String>().trim().to_string());
        let total_credits = doc.select(&credits_sel).next().map(|v| v.text().collect::<String>().trim().to_string());

        let mut cards: Vec<CardInfo> = Vec::new();
        for card in doc.select(&card_sel) {
            let name = card.select(&name_sel).next().map(|n| n.text().collect::<String>().trim().to_string()).unwrap_or_default();
            let image = card.select(&img_sel).next().and_then(|i| i.value().attr("src")).unwrap_or("").to_string();
            // per-card credits might be absent; try selector within card
            let credits = card.select(&credits_sel).next().map(|v| v.text().collect::<String>().trim().to_string());
            cards.push(CardInfo { name, image, credits });
        }

        let info = GameInfo {
            title,
            total_credits,
            cards,
        };

        // serialize to JSON string
        match serde_json::to_string(&info) {
            Ok(s) => Ok(JsValue::from_str(&s)),
            Err(e) => {
                let err = serde_json::json!({ "error": format!("serialize failed: {}", e) });
                Ok(JsValue::from_str(&err.to_string()))
            }
        }
    })
    }
                      
