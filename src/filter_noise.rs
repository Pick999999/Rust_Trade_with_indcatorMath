use crate::full_analysis_ver2::FullAnalysisResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct NoiseFilterConfig {
    pub is_check_noise: String, // "yes" or "no"
    pub alt_color: String,      // "yes" or "no"
    pub flat: String,           // "yes" or "no"
    pub flat_case: String,      // "Case1", "Case2", ... "Case7"
    pub gap: String,            // "yes" or "no"
    // ═══ New Noise Filters (Loss Streak Fix) ═══
    pub check_adx: String,         // "yes" or "no" — กรองตลาดไม่มีเทรนด์
    pub adx_threshold: f64,        // default 25.0 — ADX < threshold = weak trend
    pub check_choppy: String,      // "yes" or "no" — กรองตลาด sideway
    pub choppy_threshold: f64,     // default 50.0 — Choppy > threshold = sideway
    pub check_range: String,       // "yes" or "no" — กรองเมื่ออยู่ใน range box
    pub check_bb_squeeze: String,  // "yes" or "no" — กรองเมื่อ volatility ต่ำ
    pub bb_bandwidth_threshold: f64, // default 0.5 — BB bandwidth < threshold = squeeze
}

#[allow(non_snake_case)]
pub fn check_noise_filter(data: &FullAnalysisResult, config: &NoiseFilterConfig) -> (bool, String) {
    let check_noise = config.is_check_noise.to_lowercase();
    if check_noise != "yes" && check_noise != "y" {
        return (false, "nochecknoise".to_string());
    }

    let alt = config.alt_color.to_lowercase();
    if alt == "yes" || alt == "y" {
        if data.is_alternating_pattern || data.is_alternating_spike || data.is_alternating_trigger {
            return (true, "alter".to_string());
        }
    }

    let flat = config.flat.to_lowercase();
    if flat == "yes" || flat == "y" {
        match config.flat_case.as_str() {
            "Case1" => {
                if data.ema_short_flat == "y" {
                    return (true, "Flat1".to_string());
                }
            }
            "Case2" => {
                if data.ema_medium_flat == "y" {
                    return (true, "Flat2".to_string());
                }
            }
            "Case3" => {
                if data.ema_long_flat == "y" {
                    return (true, "Flat3".to_string());
                }
            }
            "Case4" => {
                if data.ema_short_flat == "y" && data.ema_medium_flat == "y" {
                    return (true, "Flat4".to_string());
                }
            }
            "Case5" => {
                if data.ema_short_flat == "y" || data.ema_medium_flat == "y" {
                    return (true, "Flat5".to_string());
                }
            }
            "Case6" => {
                if data.ema_short_flat == "y" && data.ema_medium_flat == "y" && data.ema_long_flat == "y" {
                    return (true, "Flat6".to_string());
                }
            }
            "Case7" => {
                if data.ema_short_flat == "y" || data.ema_medium_flat == "y" || data.ema_long_flat == "y" {
                    return (true, "Flat7".to_string());
                }
            }
            _ => {}
        }
    }

    let gap = config.gap.to_lowercase();
    if gap == "yes" || gap == "y" {
        if data.is_medium_long_gap_occur == "y" || data.is_short_medium_gap_occur == "y" {
            return (true, "gap".to_string());
        }
    }

    // ═══ A1. ADX Filter — กรองตลาดไม่มีเทรนด์ ═══
    let adx = config.check_adx.to_lowercase();
    if adx == "yes" || adx == "y" {
        if data.adx_value < config.adx_threshold {
            return (true, format!("weak_trend(ADX={:.1}<{:.1})", data.adx_value, config.adx_threshold));
        }
    }

    // ═══ A2. Choppy Filter — กรองตลาด sideway ═══
    let choppy = config.check_choppy.to_lowercase();
    if choppy == "yes" || choppy == "y" {
        if data.choppy_indicator > config.choppy_threshold {
            return (true, format!("choppy(CI={:.1}>{:.1})", data.choppy_indicator, config.choppy_threshold));
        }
    }

    // ═══ A3. Range Detector Filter — กรองเมื่ออยู่ใน range box ═══
    let range = config.check_range.to_lowercase();
    if range == "yes" || range == "y" {
        if data.range_detector.in_range {
            return (true, "in_range".to_string());
        }
    }

    // ═══ A4. BB Squeeze Filter — กรองเมื่อ volatility ต่ำ ═══
    let bb_squeeze = config.check_bb_squeeze.to_lowercase();
    if bb_squeeze == "yes" || bb_squeeze == "y" {
        if data.is_bb_squeeze || data.bb_bandwidth < config.bb_bandwidth_threshold {
            return (true, format!("bb_squeeze(BW={:.4}<{:.4})", data.bb_bandwidth, config.bb_bandwidth_threshold));
        }
    }

    // Default to not idle if noise checks pass
    (false, "none".to_string())
}
