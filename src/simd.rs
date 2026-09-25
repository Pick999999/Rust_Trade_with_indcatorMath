// SIMD-optimized mathematical operations
// Uses explicit SIMD when available, fallback to auto-vectorization

#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
use std::arch::x86_64::*;

/// SIMD-accelerated sum (uses AVX2 when available)
#[inline]
pub fn sum_f64(data: &[f64]) -> f64 {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    unsafe {
        sum_f64_avx2(data)
    }

    #[cfg(not(all(target_arch = "x86_64", target_feature = "avx2")))]
    {
        sum_f64_scalar(data)
    }
}

/// SIMD-accelerated weighted sum
#[inline]
pub fn weighted_sum_f64(data: &[f64], weights: &[f64]) -> f64 {
    assert_eq!(data.len(), weights.len());
    
    data.iter()
        .zip(weights.iter())
        .map(|(d, w)| d * w)
        .sum()
}

/// Scalar fallback
#[inline(always)]
fn sum_f64_scalar(data: &[f64]) -> f64 {
    data.iter().sum()
}

/// AVX2-accelerated sum (4x f64 per iteration)
#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
#[inline]
unsafe fn sum_f64_avx2(data: &[f64]) -> f64 {
    let mut sum = _mm256_setzero_pd();
    let chunks = data.chunks_exact(4);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let vals = _mm256_loadu_pd(chunk.as_ptr());
        sum = _mm256_add_pd(sum, vals);
    }

    // Horizontal sum of 4 lanes
    let mut result = [0.0f64; 4];
    _mm256_storeu_pd(result.as_mut_ptr(), sum);
    
    result.iter().sum::<f64>() + remainder.iter().sum::<f64>()
}

/// Fast exponential moving average update (inline for zero overhead)
#[inline(always)]
pub fn ema_update(prev: f64, new: f64, alpha: f64) -> f64 {
    alpha * new + (1.0 - alpha) * prev
}

/// Standard deviation (cache-friendly single pass)
#[inline]
pub fn std_dev(data: &[f64], mean: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    
    let variance: f64 = data.iter()
        .map(|x| {
            let diff = x - mean;
            diff * diff
        })
        .sum::<f64>() / data.len() as f64;
    
    variance.sqrt()
}

/// Mean (optimized for cache locality)
#[inline]
pub fn mean(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    sum_f64(data) / data.len() as f64
}

// ============================================================================
// BASELINE IMPLEMENTATIONS (No SIMD) - For Performance Comparison
// ============================================================================

/// Baseline sum (no SIMD optimization)
#[inline]
pub fn sum_f64_baseline(data: &[f64]) -> f64 {
    let mut sum = 0.0;
    for &value in data {
        sum += value;
    }
    sum
}

/// Baseline weighted sum (no SIMD optimization)
#[inline]
pub fn weighted_sum_f64_baseline(data: &[f64], weights: &[f64]) -> f64 {
    assert_eq!(data.len(), weights.len());
    
    let mut sum = 0.0;
    for i in 0..data.len() {
        sum += data[i] * weights[i];
    }
    sum
}

/// Baseline mean (no SIMD optimization)
#[inline]
pub fn mean_baseline(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    sum_f64_baseline(data) / data.len() as f64
}
