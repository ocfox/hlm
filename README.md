# hlm

hyperliquid price overlay for wayland. shows a live candlestick chart in the corner of your screen — no window switching when you're trading oil futures or anything else while watching a stream or chating with friends.

layer-shell surface: click-through, semi-transparent, always on top.

<img width="3840" height="2160" alt="image" src="https://github.com/user-attachments/assets/682e4071-5298-449b-8c55-02fae89cc1b9" />

## usage

```
hlm BTC
hlm xyz:CL -c m5
```

subscribes to the hyperliquid websocket candle feed.

options:

- `-c` — interval: `m1` `m3` `m5` `m15` `m30` `h1` `h2` `h4` `h8` `h12` `d1` `d3` `w1` `M1` (default `m1`)
- `-w` — window width in logical pixels (default `160`)

requires a wayland compositor with `wlr-layer-shell` support (niri, sway, river, etc.).
