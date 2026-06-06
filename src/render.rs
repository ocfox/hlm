use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache, Weight};
use tiny_skia::{Color as SkiaColor, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

use crate::state::AppState;

pub const WIN_W: u32 = 160;
pub const WIN_H: u32 = 80;
const HEADER_H: u32 = 22;

const BG: u32    = 0xD9131722;
const GREEN: u32 = 0xFF26A69A;
const RED: u32   = 0xFFEF5350;

const LABEL_SZ: f32 = 11.0;
const PRICE_SZ: f32 = 14.0;
const PAD: f32 = 5.0;
const TEXT_Y: f32 = 2.0;

pub fn paint(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    state: &AppState,
    scale: i32,
) {
    let header_h = HEADER_H * scale as u32;
    draw_candles(pixmap, state, header_h);
    draw_header(pixmap, font_system, swash_cache, state, scale);
}

fn draw_candles(pixmap: &mut Pixmap, state: &AppState, header_h: u32) {
    pixmap.fill(u32_to_color(BG));

    let candles = &state.candles;
    if candles.is_empty() { return; }

    let w = pixmap.width() as f32;
    let h = (pixmap.height() - header_h) as f32;
    let top = header_h as f32;

    let (lo, hi) = candles.iter().fold((f64::MAX, f64::MIN), |(lo, hi), c| {
        (lo.min(c.low()), hi.max(c.high()))
    });
    let range = (hi - lo).max(1e-9);

    let slot_w = w / candles.len() as f32;
    let body_w = (slot_w * 0.6).max(1.0);
    let pad = 4.0_f32;
    let price_y = |p: f64| -> f32 { top + pad + ((hi - p) / range) as f32 * (h - 2.0 * pad) };

    for (i, candle) in candles.iter().enumerate() {
        let cx = (i as f32 + 0.5) * slot_w;
        let open = candle.open();
        let close = candle.close();
        let oy = price_y(open);
        let cy = price_y(close);
        let color = if close >= open { GREEN } else { RED };

        draw_line(pixmap, cx, price_y(candle.high()), cx, price_y(candle.low()), 2.0, color);

        let body_top = oy.min(cy);
        let body_h = (oy.max(cy) - body_top).max(1.0);
        fill_rect(pixmap, cx - body_w / 2.0, body_top, body_w, body_h, color);
    }
}

fn u32_to_color(c: u32) -> SkiaColor {
    SkiaColor::from_rgba8(
        ((c >> 16) & 0xFF) as u8,
        ((c >> 8) & 0xFF) as u8,
        (c & 0xFF) as u8,
        ((c >> 24) & 0xFF) as u8,
    )
}

fn fill_rect(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, color: u32) {
    let Some(rect) = Rect::from_xywh(x, y, w, h) else { return };
    let mut paint = Paint::default();
    paint.set_color(u32_to_color(color));
    paint.anti_alias = false;
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

fn draw_line(pixmap: &mut Pixmap, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: u32) {
    let mut pb = PathBuilder::new();
    pb.move_to(x1, y1); pb.line_to(x2, y2);
    let mut paint = Paint::default();
    paint.set_color(u32_to_color(color));
    paint.anti_alias = false;
    pixmap.stroke_path(&pb.finish().unwrap(), &paint, &Stroke { width, ..Default::default() }, Transform::identity(), None);
}

fn draw_header(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    state: &AppState,
    scale: i32,
) {
    let s = scale as f32;
    let w = pixmap.width() as f32;
    let pad = PAD * s;
    let y = TEXT_Y * s;

    let label = format!("{}  {}", state.coin, state.interval_label);
    draw_text(pixmap, font_system, swash_cache, &label, pad, y, LABEL_SZ * s, 0xFFFFFF);

    if let Some(price) = state.last_price() {
        let price_str = format_price(price, state.price_decimals);
        let pw = measure_width(font_system, &price_str, PRICE_SZ * s);
        draw_text(pixmap, font_system, swash_cache, &price_str, w - pw - pad, y, PRICE_SZ * s, 0xFFFFFF);
    }
}

fn draw_text(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color_rgb: u32,
) {
    let line_h = size * 1.2;
    let attrs = Attrs::new().family(Family::Monospace).weight(Weight::BOLD);
    let mut buf = Buffer::new(font_system, Metrics::new(size, line_h));
    buf.set_size(font_system, Some(pixmap.width() as f32), Some(line_h));
    buf.set_text(font_system, text, attrs, Shaping::Basic);
    buf.shape_until_scroll(font_system, false);

    let r = ((color_rgb >> 16) & 0xFF) as u8;
    let g = ((color_rgb >> 8) & 0xFF) as u8;
    let b = (color_rgb & 0xFF) as u8;
    let color = Color::rgba(r, g, b, 0xFF);

    let mut paint = Paint::default();
    paint.anti_alias = false;

    buf.draw(font_system, swash_cache, color, |gx, gy, gw, gh, c| {
        let Some(rect) = Rect::from_xywh(x + gx as f32, y + gy as f32, gw as f32, gh as f32) else { return; };
        paint.set_color_rgba8(c.r(), c.g(), c.b(), c.a());
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    });
}

fn measure_width(font_system: &mut FontSystem, text: &str, size: f32) -> f32 {
    let line_h = size * 1.2;
    let attrs = Attrs::new().family(Family::Monospace).weight(Weight::BOLD);
    let mut buf = Buffer::new(font_system, Metrics::new(size, line_h));
    buf.set_size(font_system, Some(4096.0), Some(line_h));
    buf.set_text(font_system, text, attrs, Shaping::Basic);
    buf.shape_until_scroll(font_system, false);
    buf.layout_runs().map(|r| r.line_w).fold(0.0_f32, f32::max)
}

pub fn format_price(p: f64, decimals: usize) -> String {
    format!("{:.prec$}", p, prec = decimals)
}
