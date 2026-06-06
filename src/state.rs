use std::collections::VecDeque;

use crate::feed::Candle;

pub const CANDLE_COUNT: usize = 10;

pub struct AppState {
    pub coin: String,
    pub interval_label: String,
    pub candles: VecDeque<Candle>,
    pub price_decimals: usize,
    pub dirty: bool,
}

impl AppState {
    pub fn new(coin: String, interval_label: String) -> Self {
        Self {
            coin,
            interval_label,
            candles: VecDeque::with_capacity(CANDLE_COUNT + 1),
            price_decimals: 0,
            dirty: true,
        }
    }

    pub fn push(&mut self, candle: Candle) {
        // Learn precision from incoming price strings (take max, never shrink).
        for s in [&candle.o, &candle.h, &candle.l, &candle.c] {
            self.price_decimals = self.price_decimals.max(decimal_places(s));
        }

        if let Some(last) = self.candles.back_mut() {
            if last.t == candle.t {
                *last = candle;
                self.dirty = true;
                return;
            }
        }
        if self.candles.len() == CANDLE_COUNT {
            self.candles.pop_front();
        }
        self.candles.push_back(candle);
        self.dirty = true;
    }

    pub fn last_price(&self) -> Option<f64> {
        self.candles.back().map(|c| c.close())
    }
}

fn decimal_places(s: &str) -> usize {
    s.find('.')
        .map(|i| s[i + 1..].trim_end_matches('0').len())
        .unwrap_or(0)
}
