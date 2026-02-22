/**
 * CLF SIMD kernels — C header for fixed ABI.
 * All sizes in elements (f32 count). Caller ensures non-null pointers and consistent lengths.
 */
#ifndef CLF_SIMD_H
#define CLF_SIMD_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

void clf_simd_add(float *out, const float *a, const float *b, size_t count);
void clf_simd_relu(float *out, const float *x, size_t count);
void clf_simd_batchnorm(
    float *out,
    const float *x,
    const float *scale,
    const float *bias,
    const float *mean,
    const float *var,
    size_t n, size_t c, size_t h, size_t w);
void clf_simd_global_avg_pool(
    float *out,
    const float *inp,
    size_t n, size_t c, size_t h, size_t w);
void clf_simd_conv(
    float *out,
    const float *inp,
    const float *weights,
    const float *bias,
    size_t n, size_t c_in, size_t h, size_t w,
    size_t c_out, size_t k_h, size_t k_w,
    size_t stride_h, size_t stride_w,
    size_t pad_h, size_t pad_w);
void clf_simd_matmul(
    float *out,
    const float *a,
    const float *b,
    size_t m, size_t k, size_t n);

#ifdef __cplusplus
}
#endif

#endif /* CLF_SIMD_H */
