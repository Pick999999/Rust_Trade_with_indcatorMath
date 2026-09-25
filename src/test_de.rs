use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IndicatorsConfigGroupMain {
    #[serde(rename = "adxPeriod")]
    pub adx_period: u32,
    #[serde(rename = "altCandleAtrMultiplier")]
    pub alt_candle_atr_multiplier: Option<f64>,
    #[serde(rename = "atrMulti")]
    pub atr_multi: f64,
    #[serde(rename = "atrPeriod")]
    pub atr_period: u32,
    #[serde(rename = "bbPeriod")]
    pub bb_period: u32,
    #[serde(rename = "ciPeriod")]
    pub ci_period: u32,
    #[serde(rename = "smcPeriod")]
    pub smc_period: u32,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IndicatorsConfigGroupVer2 {
    pub adx_period: u32,
    pub atr_period: u32,
    pub atr_multi: f64,
    pub bb_period: u32,
    pub ci_period: u32,
    pub smc_period: u32,
}

fn main() {
    let json = r#"{
      "adxPeriod": 14,
      "altCandleAtrMultiplier": 0.5,
      "atrMulti": 1.3,
      "atrPeriod": 7,
      "bbPeriod": 20,
      "ciPeriod": 20,
      "smcPeriod": 50
    }"#;
    
    let parsed_ver2: Result<IndicatorsConfigGroupVer2, _> = serde_json::from_str(json);
    println!("Parsed: {:?}", parsed_ver2);
}
