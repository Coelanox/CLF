//! SIMD kernels for 6 neural-network ops. Fixed C ABI for use from C/Rust or a minimal runtime (e.g. CLFC).
//!
//! **ABI summary** (all sizes in **elements**, not bytes; pointers are non-null, buffers pre-allocated by caller):
//!
//! | Op            | Arguments | Notes |
//! |---------------|-----------|--------|
//! | Add           | out, a, b, count | 1D; count = len(a)=len(b)=len(out). |
//! | ReLU          | out, x, count | 1D. |
//! | BatchNorm     | out, x, scale, bias, mean, var, n, c, h, w | x/out NCHW; scale/bias/mean/var length C. |
//! | GlobalAvgPool | out, inp, n, c, h, w | inp NCHW (n*c*h*w); out (n*c). |
//! | Convolution   | out, inp, weights, bias, n, c_in, h, w, c_out, k_h, k_w, stride_h, stride_w, pad_h, pad_w | bias may be null; weights [c_out, c_in, k_h, k_w]. |
//! | MatMul        | out, a, b, m, k, n | a [M,K], b [K,N], out [M,N]; row-major. |

//! For a true no_std / no-OS build: use nightly, `rustup component add rust-src`, and
//! `cargo build -Z build-std=core,panic_abort --release` with a panic handler.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

#[cfg(not(feature = "std"))]
use libm::sqrtf;
#[cfg(feature = "std")]
fn sqrtf(x: f32) -> f32 {
    x.sqrt()
}

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

const LANE_F32: usize = 8; // AVX2 = 256 bits = 8 x f32

// --------------- Add ---------------
// ABI: (out, a, b, count). out[i] = a[i] + b[i]. count in elements. All ptrs non-null, count consistent.

#[inline(always)]
fn add_scalar(out: *mut f32, a: *const f32, b: *const f32, count: usize) {
    for i in 0..count {
        unsafe { *out.add(i) = *a.add(i) + *b.add(i) };
    }
}

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
#[inline(always)]
unsafe fn add_avx2(out: *mut f32, a: *const f32, b: *const f32, count: usize) {
    let mut i = 0usize;
    while i + LANE_F32 <= count {
        let va = _mm256_loadu_ps(a.add(i));
        let vb = _mm256_loadu_ps(b.add(i));
        _mm256_storeu_ps(out.add(i), _mm256_add_ps(va, vb));
        i += LANE_F32;
    }
    add_scalar(out.add(i), a.add(i), b.add(i), count - i);
}

/// Add: out[i] = a[i] + b[i]. Same shape, f32.
/// ABI: out (mut), a (in), b (in), count (elements). No alloc; caller guarantees non-null and consistent lengths.
#[no_mangle]
pub unsafe extern "C" fn clf_simd_add(out: *mut f32, a: *const f32, b: *const f32, count: usize) {
    if count == 0 {
        return;
    }
    #[cfg(all(target_arch = "x86_64", feature = "avx2"))]
    {
        add_avx2(out, a, b, count);
    }
    #[cfg(not(all(target_arch = "x86_64", feature = "avx2")))]
    {
        add_scalar(out, a, b, count);
    }
}

// --------------- ReLU ---------------
// ABI: (out, x, count). out[i] = max(0, x[i]). count in elements.

#[inline(always)]
fn relu_scalar(out: *mut f32, x: *const f32, count: usize) {
    for i in 0..count {
        let v = unsafe { *x.add(i) };
        unsafe { *out.add(i) = if v > 0.0 { v } else { 0.0 } };
    }
}

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
#[inline(always)]
unsafe fn relu_avx2(out: *mut f32, x: *const f32, count: usize) {
    let zero = _mm256_setzero_ps();
    let mut i = 0usize;
    while i + LANE_F32 <= count {
        let v = _mm256_loadu_ps(x.add(i));
        _mm256_storeu_ps(out.add(i), _mm256_max_ps(v, zero));
        i += LANE_F32;
    }
    relu_scalar(out.add(i), x.add(i), count - i);
}

/// ReLU: out[i] = max(0, x[i]). f32.
/// ABI: out (mut), x (in), count (elements).
#[no_mangle]
pub unsafe extern "C" fn clf_simd_relu(out: *mut f32, x: *const f32, count: usize) {
    if count == 0 {
        return;
    }
    #[cfg(all(target_arch = "x86_64", feature = "avx2"))]
    {
        relu_avx2(out, x, count);
    }
    #[cfg(not(all(target_arch = "x86_64", feature = "avx2")))]
    {
        relu_scalar(out, x, count);
    }
}

// --------------- BatchNorm ---------------
// out[i] = scale[c] * (x[i] - mean[c]) / sqrt(var[c] + eps) + bias[c]. NCHW; channels are the second dim (C).
// ABI: out, x, scale, bias, mean, var (each length C), n, c, h, w. x/out layout: n*C*H*W, index = n*C*H*W + c*H*W + h*W + w.

const BN_EPS: f32 = 1e-5;

#[inline(always)]
#[allow(dead_code)] // used when avx2 disabled or non-x86_64
fn batchnorm_scalar(
    out: *mut f32,
    x: *const f32,
    scale: *const f32,
    bias: *const f32,
    mean: *const f32,
    var: *const f32,
    n: usize,
    c: usize,
    h: usize,
    w: usize,
) {
    let hw = h * w;
    let chw = c * hw;
    let _nchw = n * c * hw;
    for ni in 0..n {
        for ci in 0..c {
            let sc = unsafe { *scale.add(ci) };
            let bi = unsafe { *bias.add(ci) };
            let me = unsafe { *mean.add(ci) };
            let va = unsafe { *var.add(ci) };
            let inv_std = 1.0 / sqrtf(va + BN_EPS);
            let base = ni * chw + ci * hw;
            for j in 0..hw {
                let idx = base + j;
                let v = unsafe { *x.add(idx) };
                unsafe { *out.add(idx) = sc * (v - me) * inv_std + bi };
            }
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
#[inline(always)]
unsafe fn batchnorm_avx2(
    out: *mut f32,
    x: *const f32,
    scale: *const f32,
    bias: *const f32,
    mean: *const f32,
    var: *const f32,
    n: usize,
    c: usize,
    h: usize,
    w: usize,
) {
    let hw = h * w;
    let chw = c * hw;
    for ni in 0..n {
        for ci in 0..c {
            let sc = _mm256_set1_ps(*scale.add(ci));
            let bi = _mm256_set1_ps(*bias.add(ci));
            let me = _mm256_set1_ps(*mean.add(ci));
            let va = *var.add(ci) + BN_EPS;
            let inv_std = _mm256_set1_ps(1.0 / sqrtf(va));
            let base = ni * chw + ci * hw;
            let mut j = 0usize;
            while j + LANE_F32 <= hw {
                let idx = base + j;
                let v = _mm256_loadu_ps(x.add(idx));
                let t = _mm256_mul_ps(_mm256_mul_ps(_mm256_sub_ps(v, me), inv_std), sc);
                _mm256_storeu_ps(out.add(idx), _mm256_add_ps(t, bi));
                j += LANE_F32;
            }
            for jj in j..hw {
                let idx = base + jj;
                let v = *x.add(idx);
                *out.add(idx) = (*scale.add(ci)) * (v - *mean.add(ci)) / sqrtf(va) + *bias.add(ci);
            }
        }
    }
}

/// BatchNorm: out = scale * (x - mean) / sqrt(var + eps) + bias. One scale, bias, mean, var per channel (NCHW).
/// ABI: out, x, scale, bias, mean, var (C elements each), n, c, h, w.
#[no_mangle]
pub unsafe extern "C" fn clf_simd_batchnorm(
    out: *mut f32,
    x: *const f32,
    scale: *const f32,
    bias: *const f32,
    mean: *const f32,
    var: *const f32,
    n: usize,
    c: usize,
    h: usize,
    w: usize,
) {
    if n == 0 || c == 0 || h == 0 || w == 0 {
        return;
    }
    #[cfg(all(target_arch = "x86_64", feature = "avx2"))]
    {
        batchnorm_avx2(out, x, scale, bias, mean, var, n, c, h, w);
    }
    #[cfg(not(all(target_arch = "x86_64", feature = "avx2")))]
    {
        batchnorm_scalar(out, x, scale, bias, mean, var, n, c, h, w);
    }
}

// --------------- GlobalAvgPool ---------------
// out[n,c] = mean over H,W of inp[n,c,:,:]. Input NCHW (n*c*h*w), output (n*c).
// ABI: out, inp, n, c, h, w. All sizes in elements.

#[allow(dead_code)]
fn global_avg_pool_scalar(out: *mut f32, inp: *const f32, n: usize, c: usize, h: usize, w: usize) {
    let hw = h * w;
    let chw = c * hw;
    for ni in 0..n {
        for ci in 0..c {
            let base = ni * chw + ci * hw;
            let mut sum = 0.0f32;
            for j in 0..hw {
                sum += unsafe { *inp.add(base + j) };
            }
            unsafe { *out.add(ni * c + ci) = sum / hw as f32 };
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
unsafe fn global_avg_pool_avx2(
    out: *mut f32,
    inp: *const f32,
    n: usize,
    c: usize,
    h: usize,
    w: usize,
) {
    let hw = h * w;
    let chw = c * hw;
    let inv_hw = 1.0 / hw as f32;
    for ni in 0..n {
        for ci in 0..c {
            let base = ni * chw + ci * hw;
            let mut acc = _mm256_setzero_ps();
            let mut j = 0usize;
            while j + LANE_F32 <= hw {
                acc = _mm256_add_ps(acc, _mm256_loadu_ps(inp.add(base + j)));
                j += LANE_F32;
            }
            // horizontal sum of acc
            let shuf = _mm256_shuffle_ps(acc, acc, 0b10_11_00_01);
            let sum1 = _mm256_add_ps(acc, shuf);
            let shuf2 = _mm256_permute2f128_ps(sum1, sum1, 1);
            let sum2 = _mm256_add_ps(sum1, shuf2);
            let shuf3 = _mm256_shuffle_ps(sum2, sum2, 0b01_00_11_10);
            let sum3 = _mm256_add_ps(sum2, shuf3);
            let s = _mm256_cvtss_f32(sum3);
            let mut scalar_sum = s;
            for jj in j..hw {
                scalar_sum += *inp.add(base + jj);
            }
            *out.add(ni * c + ci) = scalar_sum * inv_hw;
        }
    }
}

/// GlobalAvgPool: out[n,c] = mean over H,W of inp. Input [N,C,H,W], output [N,C]. f32.
/// ABI: out (n*c), inp (n*c*h*w), n, c, h, w (all elements).
#[no_mangle]
pub unsafe extern "C" fn clf_simd_global_avg_pool(
    out: *mut f32,
    inp: *const f32,
    n: usize,
    c: usize,
    h: usize,
    w: usize,
) {
    if n == 0 || c == 0 || h == 0 || w == 0 {
        return;
    }
    #[cfg(all(target_arch = "x86_64", feature = "avx2"))]
    {
        global_avg_pool_avx2(out, inp, n, c, h, w);
    }
    #[cfg(not(all(target_arch = "x86_64", feature = "avx2")))]
    {
        global_avg_pool_scalar(out, inp, n, c, h, w);
    }
}

// --------------- Convolution ---------------
// 2D NCHW: inp [N, c_in, H, W], weights [c_out, c_in, k_h, k_w], out [N, c_out, H_out, W_out].
// H_out = (H + 2*pad_h - k_h) / stride_h + 1; same for W.
// ABI: out, inp, weights, bias (may be null), n, c_in, h, w, c_out, k_h, k_w, stride_h, stride_w, pad_h, pad_w.
// Weights layout: index (co, ci, kh, kw) = co*(c_in*k_h*k_w) + ci*(k_h*k_w) + kh*k_w + kw.

unsafe fn conv_scalar(
    out: *mut f32,
    inp: *const f32,
    weights: *const f32,
    bias: *const f32,
    n: usize,
    c_in: usize,
    h: usize,
    w: usize,
    c_out: usize,
    k_h: usize,
    k_w: usize,
    stride_h: usize,
    stride_w: usize,
    pad_h: usize,
    pad_w: usize,
) {
    let h_out = (h + 2 * pad_h - k_h) / stride_h + 1;
    let w_out = (w + 2 * pad_w - k_w) / stride_w + 1;
    let in_hw = h * w;
    let in_chw = c_in * in_hw;
    let w_k = k_h * k_w;
    let w_ck = c_in * w_k;
    for ni in 0..n {
        for co in 0..c_out {
            for ho in 0..h_out {
                for wo in 0..w_out {
                    let mut sum = if bias.is_null() {
                        0.0
                    } else {
                        *bias.add(co)
                    };
                    for ci in 0..c_in {
                        for kh in 0..k_h {
                            for kw in 0..k_w {
                                let hi = (ho * stride_h).wrapping_add(kh).wrapping_sub(pad_h);
                                let wi = (wo * stride_w).wrapping_add(kw).wrapping_sub(pad_w);
                                if hi < h && wi < w {
                                    let in_idx = ni * in_chw + ci * in_hw + hi * w + wi;
                                    let w_idx = co * w_ck + ci * w_k + kh * k_w + kw;
                                    sum += *inp.add(in_idx) * *weights.add(w_idx);
                                }
                            }
                        }
                    }
                    let out_idx = ni * c_out * h_out * w_out + co * h_out * w_out + ho * w_out + wo;
                    *out.add(out_idx) = sum;
                }
            }
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
unsafe fn conv_avx2(
    out: *mut f32,
    inp: *const f32,
    weights: *const f32,
    bias: *const f32,
    n: usize,
    c_in: usize,
    h: usize,
    w: usize,
    c_out: usize,
    k_h: usize,
    k_w: usize,
    stride_h: usize,
    stride_w: usize,
    pad_h: usize,
    pad_w: usize,
) {
    // Inner dot product has non-contiguous indices (per output pixel); use scalar. Still benefit from AVX2 build.
    conv_scalar(
        out, inp, weights, bias, n, c_in, h, w, c_out, k_h, k_w, stride_h, stride_w, pad_h, pad_w,
    );
}

/// Convolution 2D NCHW. Supports 1x1 and 3x3 (and any k_h, k_w). Optional bias (null = no bias).
/// ABI: out, inp, weights, bias (or null), n, c_in, h, w, c_out, k_h, k_w, stride_h, stride_w, pad_h, pad_w.
#[no_mangle]
pub unsafe extern "C" fn clf_simd_conv(
    out: *mut f32,
    inp: *const f32,
    weights: *const f32,
    bias: *const f32,
    n: usize,
    c_in: usize,
    h: usize,
    w: usize,
    c_out: usize,
    k_h: usize,
    k_w: usize,
    stride_h: usize,
    stride_w: usize,
    pad_h: usize,
    pad_w: usize,
) {
    if n == 0 || c_out == 0 || c_in == 0 {
        return;
    }
    #[cfg(all(target_arch = "x86_64", feature = "avx2"))]
    {
        conv_avx2(
            out, inp, weights, bias, n, c_in, h, w, c_out, k_h, k_w, stride_h, stride_w, pad_h, pad_w,
        );
    }
    #[cfg(not(all(target_arch = "x86_64", feature = "avx2")))]
    {
        conv_scalar(
            out, inp, weights, bias, n, c_in, h, w, c_out, k_h, k_w, stride_h, stride_w, pad_h, pad_w,
        );
    }
}

// --------------- MatMul ---------------
// out = A @ B. A [M,K], B [K,N], out [M,N]. Row-major: A[i,k], B[k,j], out[i,j].
// ABI: out, a, b, m, k, n (all sizes in elements).

#[allow(dead_code)]
unsafe fn matmul_scalar(out: *mut f32, a: *const f32, b: *const f32, m: usize, k: usize, n: usize) {
    for i in 0..m {
        for j in 0..n {
            let mut sum = 0.0f32;
            for p in 0..k {
                sum += *a.add(i * k + p) * *b.add(p * n + j);
            }
            *out.add(i * n + j) = sum;
        }
    }
}

#[cfg(all(target_arch = "x86_64", feature = "avx2"))]
unsafe fn matmul_avx2(out: *mut f32, a: *const f32, b: *const f32, m: usize, k: usize, n: usize) {
    for i in 0..m {
        let mut j = 0usize;
        while j + LANE_F32 <= n {
            let mut acc = _mm256_setzero_ps();
            for p in 0..k {
                let av = _mm256_set1_ps(*a.add(i * k + p));
                let b_idx = p * n + j;
                let bv = _mm256_loadu_ps(b.add(b_idx));
                acc = _mm256_fmadd_ps(av, bv, acc);
            }
            _mm256_storeu_ps(out.add(i * n + j), acc);
            j += LANE_F32;
        }
        for j in j..n {
            let mut sum = 0.0f32;
            for p in 0..k {
                sum += *a.add(i * k + p) * *b.add(p * n + j);
            }
            *out.add(i * n + j) = sum;
        }
    }
}

/// MatMul: out = A @ B. A [M,K], B [K,N], out [M,N]. f32, row-major.
/// ABI: out, a, b, m, k, n (elements).
#[no_mangle]
pub unsafe extern "C" fn clf_simd_matmul(
    out: *mut f32,
    a: *const f32,
    b: *const f32,
    m: usize,
    k: usize,
    n: usize,
) {
    if m == 0 || n == 0 || k == 0 {
        return;
    }
    #[cfg(all(target_arch = "x86_64", feature = "avx2"))]
    {
        matmul_avx2(out, a, b, m, k, n);
    }
    #[cfg(not(all(target_arch = "x86_64", feature = "avx2")))]
    {
        matmul_scalar(out, a, b, m, k, n);
    }
}

// =============================================================================
// CLFE uniform 6-arg ABI wrappers (input_ptr, output_ptr, weights_ptr, in_len, out_len, w_len).
// These are the entry points packed into .clfc; each blob is self-contained (kernels inlined).
// x86-64 SysV: rdi, rsi, rdx, rcx, r8, r9.
// =============================================================================

/// CLFE 6-arg entry for Add. Plan: in=input_ptr, w=weights_ptr (second addend), out=output_ptr, count=in_len.
/// Calls inner add_* so blob is self-contained (no external symbol).
#[link_section = ".text.clf_abi6_add"]
#[no_mangle]
pub unsafe extern "C" fn clf_abi6_add(
    input_ptr: *const f32,
    output_ptr: *mut f32,
    weights_ptr: *const f32,
    in_len: usize,
    _out_len: usize,
    _w_len: usize,
) {
    if in_len == 0 {
        return;
    }
    #[cfg(all(target_arch = "x86_64", feature = "avx2"))]
    add_avx2(output_ptr, input_ptr, weights_ptr, in_len);
    #[cfg(not(all(target_arch = "x86_64", feature = "avx2")))]
    add_scalar(output_ptr, input_ptr, weights_ptr, in_len);
}

/// CLFE 6-arg entry for ReLU. Plan: in=input_ptr, out=output_ptr, count=in_len. Weights unused.
#[link_section = ".text.clf_abi6_relu"]
#[no_mangle]
pub unsafe extern "C" fn clf_abi6_relu(
    input_ptr: *const f32,
    output_ptr: *mut f32,
    _weights_ptr: *const f32,
    in_len: usize,
    _out_len: usize,
    _w_len: usize,
) {
    if in_len == 0 {
        return;
    }
    #[cfg(all(target_arch = "x86_64", feature = "avx2"))]
    relu_avx2(output_ptr, input_ptr, in_len);
    #[cfg(not(all(target_arch = "x86_64", feature = "avx2")))]
    relu_scalar(output_ptr, input_ptr, in_len);
}

/// CLFE 6-arg entry for BatchNorm. Weights: scale,bias,mean,var each c elements => w_len=4*c. If w_len==0, identity copy.
#[link_section = ".text.clf_abi6_batchnorm"]
#[no_mangle]
pub unsafe extern "C" fn clf_abi6_batchnorm(
    input_ptr: *const f32,
    output_ptr: *mut f32,
    weights_ptr: *const f32,
    in_len: usize,
    _out_len: usize,
    w_len: usize,
) {
    if in_len == 0 {
        return;
    }
    if w_len == 0 {
        if input_ptr != output_ptr {
            core::ptr::copy_nonoverlapping(input_ptr, output_ptr, in_len);
        }
        return;
    }
    let c = w_len / 4;
    if c == 0 {
        return;
    }
    let n = 1usize;
    let hw = in_len / c;
    let h = (hw as f64).sqrt() as usize;
    let w = if h > 0 { hw / h } else { 0 };
    let scale = weights_ptr;
    let bias = weights_ptr.add(c);
    let mean = weights_ptr.add(2 * c);
    let var = weights_ptr.add(3 * c);
    #[cfg(all(target_arch = "x86_64", feature = "avx2"))]
    batchnorm_avx2(output_ptr, input_ptr, scale, bias, mean, var, n, c, h, w);
    #[cfg(not(all(target_arch = "x86_64", feature = "avx2")))]
    batchnorm_scalar(output_ptr, input_ptr, scale, bias, mean, var, n, c, h, w);
}

/// CLFE 6-arg entry for GlobalAvgPool. in_len=n*c*h*w, out_len=n*c => n=1, c=out_len, h*w=in_len/c.
#[link_section = ".text.clf_abi6_global_avg_pool"]
#[no_mangle]
pub unsafe extern "C" fn clf_abi6_global_avg_pool(
    input_ptr: *const f32,
    output_ptr: *mut f32,
    _weights_ptr: *const f32,
    in_len: usize,
    out_len: usize,
    _w_len: usize,
) {
    if out_len == 0 {
        return;
    }
    let n = 1usize;
    let c = out_len;
    let hw = in_len / c;
    let h = (hw as f64).sqrt() as usize;
    let w = if h > 0 { hw / h } else { 0 };
    #[cfg(all(target_arch = "x86_64", feature = "avx2"))]
    global_avg_pool_avx2(output_ptr, input_ptr, n, c, h, w);
    #[cfg(not(all(target_arch = "x86_64", feature = "avx2")))]
    global_avg_pool_scalar(output_ptr, input_ptr, n, c, h, w);
}

/// CLFE 6-arg entry for Convolution. Derive c_in,c_out,h,w,k_h,k_w from in_len,out_len,w_len; same logic as executor.
#[link_section = ".text.clf_abi6_conv"]
#[no_mangle]
pub unsafe extern "C" fn clf_abi6_conv(
    input_ptr: *const f32,
    output_ptr: *mut f32,
    weights_ptr: *const f32,
    in_len: usize,
    out_len: usize,
    w_len: usize,
) {
    if in_len == 0 || out_len == 0 || w_len == 0 {
        return;
    }
    let n = 1usize;
    let (c_in, c_out, h, w, k_h, k_w) = match derive_conv_shapes(in_len, out_len, w_len) {
        Some(t) => t,
        None => return,
    };
    let pad_h = (k_h - 1) / 2;
    let pad_w = (k_w - 1) / 2;
    #[cfg(all(target_arch = "x86_64", feature = "avx2"))]
    conv_avx2(
        output_ptr, input_ptr, weights_ptr, core::ptr::null(),
        n, c_in, h, w, c_out, k_h, k_w, 1, 1, pad_h, pad_w,
    );
    #[cfg(not(all(target_arch = "x86_64", feature = "avx2")))]
    conv_scalar(
        output_ptr, input_ptr, weights_ptr, core::ptr::null(),
        n, c_in, h, w, c_out, k_h, k_w, 1, 1, pad_h, pad_w,
    );
}

/// Derive (c_in, c_out, h, w, k_h, k_w) from in_len, out_len, w_len. Returns None if invalid.
fn derive_conv_shapes(
    in_len: usize,
    out_len: usize,
    w_len: usize,
) -> Option<(usize, usize, usize, usize, usize, usize)> {
    for c_in_candidate in 1..=in_len {
        if in_len % c_in_candidate != 0 {
            continue;
        }
        let spatial = in_len / c_in_candidate;
        if out_len % spatial != 0 {
            continue;
        }
        let c_out_candidate = out_len / spatial;
        let weight_elements = c_in_candidate * c_out_candidate;
        if weight_elements == 0 || w_len % weight_elements != 0 {
            continue;
        }
        let k_prod = w_len / weight_elements;
        let k_h_cand = (k_prod as f64).sqrt() as usize;
        let k_w_cand = if k_h_cand > 0 { k_prod / k_h_cand } else { 0 };
        if k_h_cand == 0 || k_w_cand == 0 || k_h_cand * k_w_cand != k_prod {
            continue;
        }
        let h_cand = (spatial as f64).sqrt() as usize;
        let w_cand = if h_cand > 0 { spatial / h_cand } else { 0 };
        if h_cand * w_cand != spatial {
            continue;
        }
        return Some((
            c_in_candidate,
            c_out_candidate,
            h_cand,
            w_cand,
            k_h_cand,
            k_w_cand,
        ));
    }
    None
}

/// CLFE 6-arg entry for MatMul. in_len=m*k, w_len=k*n, out_len=m*n => k=sqrt(in_len*w_len/out_len), m=in_len/k, n=out_len/m.
#[link_section = ".text.clf_abi6_matmul"]
#[no_mangle]
pub unsafe extern "C" fn clf_abi6_matmul(
    input_ptr: *const f32,
    output_ptr: *mut f32,
    weights_ptr: *const f32,
    in_len: usize,
    out_len: usize,
    w_len: usize,
) {
    if out_len == 0 {
        return;
    }
    let k_sq = in_len * w_len / out_len;
    let k = (k_sq as f64).sqrt() as usize;
    if k == 0 {
        return;
    }
    let m = in_len / k;
    let n = out_len / m;
    if m == 0 || n == 0 {
        return;
    }
    #[cfg(all(target_arch = "x86_64", feature = "avx2"))]
    matmul_avx2(output_ptr, input_ptr, weights_ptr, m, k, n);
    #[cfg(not(all(target_arch = "x86_64", feature = "avx2")))]
    matmul_scalar(output_ptr, input_ptr, weights_ptr, m, k, n);
}
