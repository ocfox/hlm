use clap::Parser;
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "hlm", about = "Hyperliquid price monitor")]
pub struct Args {
    /// Asset to monitor (e.g. BTC, xyz:CL, xyz:SILVER)
    pub coin: String,

    /// Candle interval [m1 m3 m5 m15 m30 h1 h2 h4 h8 h12 d1 d3 w1 M1]
    #[arg(short = 'c', default_value = "m1")]
    pub interval: Interval,

    /// Window width in logical pixels
    #[arg(short = 'w', long, default_value_t = crate::render::WIN_W)]
    pub width: u32,
}

#[derive(Clone, Debug)]
pub enum Interval {
    M1, M3, M5, M15, M30,
    H1, H2, H4, H8, H12,
    D1, D3, W1, Mo1,
}

impl Interval {
    pub fn to_hl(&self) -> &'static str {
        match self {
            Interval::M1  => "1m",
            Interval::M3  => "3m",
            Interval::M5  => "5m",
            Interval::M15 => "15m",
            Interval::M30 => "30m",
            Interval::H1  => "1h",
            Interval::H2  => "2h",
            Interval::H4  => "4h",
            Interval::H8  => "8h",
            Interval::H12 => "12h",
            Interval::D1  => "1d",
            Interval::D3  => "3d",
            Interval::W1  => "1w",
            Interval::Mo1 => "1M",
        }
    }

    /// Milliseconds per candle, used for bootstrap startTime calculation.
    pub fn millis(&self) -> u64 {
        match self {
            Interval::M1  => 60_000,
            Interval::M3  => 180_000,
            Interval::M5  => 300_000,
            Interval::M15 => 900_000,
            Interval::M30 => 1_800_000,
            Interval::H1  => 3_600_000,
            Interval::H2  => 7_200_000,
            Interval::H4  => 14_400_000,
            Interval::H8  => 28_800_000,
            Interval::H12 => 43_200_000,
            Interval::D1  => 86_400_000,
            Interval::D3  => 259_200_000,
            Interval::W1  => 604_800_000,
            Interval::Mo1 => 2_592_000_000,
        }
    }
}

impl FromStr for Interval {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "m1"  => Ok(Interval::M1),
            "m3"  => Ok(Interval::M3),
            "m5"  => Ok(Interval::M5),
            "m15" => Ok(Interval::M15),
            "m30" => Ok(Interval::M30),
            "h1"  => Ok(Interval::H1),
            "h2"  => Ok(Interval::H2),
            "h4"  => Ok(Interval::H4),
            "h8"  => Ok(Interval::H8),
            "h12" => Ok(Interval::H12),
            "d1"  => Ok(Interval::D1),
            "d3"  => Ok(Interval::D3),
            "w1"  => Ok(Interval::W1),
            "M1"  => Ok(Interval::Mo1),
            _ => Err(format!("unknown interval '{s}', try m1 m5 h1 etc.")),
        }
    }
}
