use std::time::{Duration, SystemTime, UNIX_EPOCH};

use calloop::channel::Sender;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const INFO_URL: &str = "https://api.hyperliquid.xyz/info";
const WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
const CANDLE_COUNT: u64 = 10;

#[derive(Debug, Clone, Deserialize)]
pub struct Candle {
    pub t: u64,
    pub o: String,
    pub h: String,
    pub l: String,
    pub c: String,
}

impl Candle {
    pub fn open(&self)  -> f64 { self.o.parse().unwrap_or(0.0) }
    pub fn high(&self)  -> f64 { self.h.parse().unwrap_or(0.0) }
    pub fn low(&self)   -> f64 { self.l.parse().unwrap_or(0.0) }
    pub fn close(&self) -> f64 { self.c.parse().unwrap_or(0.0) }
}

#[derive(Debug, Deserialize)]
struct WsMsg {
    channel: String,
    data: serde_json::Value,
}

pub async fn fetch(coin: &str, interval_hl: &str, interval_ms: u64) -> Vec<Candle> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let start = now.saturating_sub(interval_ms * CANDLE_COUNT);

    let body = json!({
        "type": "candleSnapshot",
        "req": { "coin": coin, "interval": interval_hl, "startTime": start, "endTime": now }
    });

    let resp = match reqwest::Client::new().post(INFO_URL).json(&body).send().await {
        Ok(r) => r,
        Err(e) => { eprintln!("bootstrap error: {e}"); return vec![]; }
    };

    match resp.json::<Vec<Candle>>().await {
        Ok(mut v) => {
            v.sort_by_key(|c| c.t);
            v.into_iter().rev().take(CANDLE_COUNT as usize).rev().collect()
        }
        Err(e) => { eprintln!("bootstrap parse error: {e}"); vec![] }
    }
}

pub async fn connect(coin: String, interval: String, tx: Sender<Candle>) {
    let mut backoff = Duration::from_secs(1);

    loop {
        match connect_async(WS_URL).await {
            Ok((mut ws, _)) => {
                backoff = Duration::from_secs(1);

                let sub = json!({
                    "method": "subscribe",
                    "subscription": { "type": "candle", "coin": coin, "interval": interval }
                });
                if ws.send(Message::Text(sub.to_string().into())).await.is_err() {
                    continue;
                }

                let mut ping = tokio::time::interval(Duration::from_secs(30));
                ping.tick().await;

                loop {
                    tokio::select! {
                        _ = ping.tick() => {
                            let _ = ws.send(Message::Text(r#"{"method":"ping"}"#.into())).await;
                        }
                        msg = ws.next() => match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Some(c) = parse_candle(&text) {
                                    if tx.send(c).is_err() { return; }
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            Some(Err(_)) => break,
                            _ => {}
                        }
                    }
                }
            }
            Err(e) => eprintln!("ws connect error: {e}"),
        }

        sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(30));
    }
}

fn parse_candle(text: &str) -> Option<Candle> {
    let msg: WsMsg = serde_json::from_str(text).ok()?;
    if msg.channel != "candle" { return None; }
    if msg.data.is_array() {
        msg.data.as_array()?.last().and_then(|v| serde_json::from_value(v.clone()).ok())
    } else {
        serde_json::from_value(msg.data).ok()
    }
}
