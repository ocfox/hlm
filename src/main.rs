use calloop::channel;
use clap::Parser;

mod cli;
mod feed;
mod render;
mod state;
mod window;

use cli::Args;

fn main() {
    let args = Args::parse();

    let coin = args.coin.clone();
    let interval_hl = args.interval.to_hl().to_string();
    let interval_ms = args.interval.millis();

    let (tx, rx) = channel::channel::<feed::Candle>();

    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                for candle in feed::fetch(&coin, &interval_hl, interval_ms).await {
                    let _ = tx.send(candle);
                }
                feed::connect(coin, interval_hl, tx).await;
            });
    });

    window::run(args, rx);
}
