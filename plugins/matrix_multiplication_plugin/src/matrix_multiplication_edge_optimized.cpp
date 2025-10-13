// 边缘端优化的矩阵乘法算法实现
// 针对8GB内存、4核CPU的资源受限环境优化

#include "matrix_multiplication_edge_optimized.hpp"
#include <algorithm>
#include <cmath>
#include <iostream>
#include <chrono>
#include <omp.h>

namespace MatrixMultiplication {

// 内存池实现
MemoryPool::MemoryPool(size_t initial_capacity)
    : pool_(initial_capacity), used_(0), free_blocks_() {}

MatrixElement* MemoryPool::allocate(size_t size) {
    // 首先尝试在空闲块中查找合适的空间
    for (auto it = free_blocks_.begin(); it != free_blocks_.end(); ++it) {
        if (it->second >= size) {
            size_t offset = it->first;
            size_t remaining = it->second - size;

            // 如果空闲块刚好合适，直接使用
            if (remaining == 0) {
                free_blocks_.erase(it);
            } else {
                // 否则分割空闲块
                it->first += size;
                it->second = remaining;
            }

            return &pool_[offset];
        }
    }

    // 如果没有找到合适的空闲块，从池尾部分配
    if (used_ + size > pool_.size()) {
        // 池不够大，扩展池
        size_t new_size = std::max(pool_.size() * 2, used_ + size);
        pool_.resize(new_size);
    }

    MatrixElement* ptr = &pool_[used_];
    used_ += size;
    return ptr;
}

void MemoryPool::deallocate(MatrixElement* ptr, size_t size) {
    if (ptr >= &pool_[0] && ptr < &pool_[pool_.size()]) {
        size_t offset = ptr - &pool_[0];
        // 简单地将释放的块加入空闲列表
        // 在实际使用中，这里应该有更复杂的合并逻辑
        free_blocks_.emplace_back(offset, size);
    }
}

void MemoryPool::reset() {
    used_ = 0;
    free_blocks_.clear();
}

// 矩阵乘法器基类实现
MatrixMultiplier::MatrixMultiplier(size_t max_memory_mb)
    : max_memory_bytes_(max_memory_mb * 1024 * 1024),
      memory_pool_(1024 * 1024), // 1M elements initial capacity
      metrics_() {}

void MatrixMultiplier::validateMatrices(const Matrix& A, const Matrix& B) {
    if (A.empty() || B.empty()) {
        throw std::invalid_argument("输入矩阵不能为空");
    }

    size_t cols_A = A[0].size();
    size_t rows_B = B.size();

    // 检查矩阵A的行长度一致性
    for (const auto& row : A) {
        if (row.size() != cols_A) {
            throw std::invalid_argument("矩阵A的行长度不一致");
        }
    }

    // 检查矩阵B的行长度一致性
    size_t cols_B = B[0].size();
    for (const auto& row : B) {
        if (row.size() != cols_B) {
            throw std::invalid_argument("矩阵B的行长度不一致");
        }
    }

    // 检查矩阵乘法维度匹配
    if (cols_A != rows_B) {
        throw std::invalid_argument("矩阵维度不匹配: A的列数(" +
                                  std::to_string(cols_A) + ") != B的行数(" +
                                  std::to_string(rows_B) + ")");
    }

    // 检查矩阵大小限制（针对边缘端优化）
    if (A.size() > EdgeConfig::MAX_MATRIX_SIZE ||
        cols_A > EdgeConfig::MAX_MATRIX_SIZE ||
        cols_B > EdgeConfig::MAX_MATRIX_SIZE) {
        throw std::invalid_argument("矩阵尺寸过大，超出边缘端处理能力限制");
    }
}

Matrix MatrixMultiplier::createResultMatrix(size_t rows, size_t cols) const {
    return Matrix(rows, MatrixRow(cols, 0.0f));
}

void MatrixMultiplier::updateMemoryUsage(size_t additional_bytes) const {
    metrics_.peak_memory_usage = std::max(metrics_.peak_memory_usage,
                                         memory_pool_.used() * sizeof(MatrixElement) + additional_bytes);
}

void MatrixMultiplier::resetMetrics() {
    metrics_ = PerformanceMetrics{};
    memory_pool_.reset();
}

bool MatrixMultiplier::checkMemoryLimit(size_t rows_A, size_t cols_A, size_t cols_B) const {
    size_t estimated_usage = estimateMemoryUsage(rows_A, cols_A, cols_B);
    return estimated_usage <= max_memory_bytes_;
}

void MatrixMultiplier::setMemoryLimit(size_t max_memory_mb) {
    max_memory_bytes_ = max_memory_mb * 1024 * 1024;
}

void MatrixMultiplier::preallocateForOperation(size_t rows_A, size_t cols_A, size_t cols_B) {
    // 预估需要的内存量并预分配
    size_t estimated_elements = (rows_A * cols_A + cols_A * cols_B + rows_A * cols_B) * 2;
    if (estimated_elements > memory_pool_.capacity()) {
        // 扩展内存池，但不要超过限制
        size_t new_capacity = std::min(estimated_elements,
                                     max_memory_bytes_ / sizeof(MatrixElement));
        // 这里暂时不实际扩展，由内存池的allocate方法处理
    }
}

// 朴素矩阵乘法实现
NaiveMultiplier::NaiveMultiplier(OptimizationLevel level)
    : MatrixMultiplier(EdgeConfig::AVAILABLE_MEMORY_MB / 4), optimization_level_(level) {}

Matrix NaiveMultiplier::multiply(const Matrix& A, const Matrix& B) {
    validateMatrices(A, B);

    resetMetrics();
    auto start_time = std::chrono::high_resolution_clock::now();

    Matrix result = (optimization_level_ >= OptimizationLevel::BASIC) ?
                   multiplyOptimized(A, B) : multiplyBasic(A, B);

    auto end_time = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
    metrics_.computation_time_ms = duration.count() / 1000.0;

    return result;
}

Matrix NaiveMultiplier::multiplyBasic(const Matrix& A, const Matrix& B) {
    size_t rows_A = A.size();
    size_t cols_A = A[0].size();
    size_t cols_B = B[0].size();

    Matrix C = createResultMatrix(rows_A, cols_B);

    // 基础实现 - 内存最省
    for (size_t i = 0; i < rows_A; ++i) {
        for (size_t j = 0; j < cols_B; ++j) {
            MatrixElement sum = 0.0f;
            for (size_t k = 0; k < cols_A; ++k) {
                sum += A[i][k] * B[k][j];
                metrics_.operations_count += 2;
                metrics_.memory_accesses += 2;
            }
            C[i][j] = sum;
            metrics_.memory_accesses += 1;
        }
    }

    return C;
}

Matrix NaiveMultiplier::multiplyOptimized(const Matrix& A, const Matrix& B) {
    size_t rows_A = A.size();
    size_t cols_A = A[0].size();
    size_t cols_B = B[0].size();

    Matrix C = createResultMatrix(rows_A, cols_B);

    // 优化实现 - 限制线程数避免过度并行
    int num_threads = (optimization_level_ >= OptimizationLevel::MODERATE) ?
                     std::min(EdgeConfig::MAX_THREADS, omp_get_max_threads()) : 1;

    #pragma omp parallel for num_threads(num_threads) if(num_threads > 1)
    for (size_t i = 0; i < rows_A; ++i) {
        for (size_t j = 0; j < cols_B; ++j) {
            MatrixElement sum = 0.0f;
            // 内循环使用SIMD（如果编译器支持）
            #pragma omp simd reduction(+:sum) if(optimization_level_ >= OptimizationLevel::MODERATE)
            for (size_t k = 0; k < cols_A; ++k) {
                sum += A[i][k] * B[k][j];
            }
            C[i][j] = sum;
        }
    }

    // 更新性能指标
    metrics_.operations_count = rows_A * cols_A * cols_B * 2;
    metrics_.memory_accesses = (rows_A * cols_A + cols_A * cols_B + rows_A * cols_B) * 2;

    return C;
}

size_t NaiveMultiplier::estimateMemoryUsage(size_t rows_A, size_t cols_A, size_t cols_B) const {
    size_t element_size = sizeof(MatrixElement);
    // 输入矩阵 + 输出矩阵 + 少量临时空间
    return (rows_A * cols_A + cols_A * cols_B + rows_A * cols_B) * element_size;
}

PerformanceMetrics NaiveMultiplier::getPerformanceMetrics() const {
    return metrics_;
}

// 分块矩阵乘法实现
TiledMultiplier::TiledMultiplier(size_t block_size, OptimizationLevel level)
    : MatrixMultiplier(EdgeConfig::AVAILABLE_MEMORY_MB / 4),
      block_size_(block_size), optimization_level_(level) {

    // 根据优化级别调整块大小
    if (optimization_level_ >= OptimizationLevel::MODERATE) {
        block_size_ = std::max(block_size_, EdgeConfig::DEFAULT_BLOCK_SIZE);
    } else {
        block_size_ = std::min(block_size_, EdgeConfig::DEFAULT_BLOCK_SIZE);
    }
}

Matrix TiledMultiplier::multiply(const Matrix& A, const Matrix& B) {
    validateMatrices(A, B);

    resetMetrics();
    auto start_time = std::chrono::high_resolution_clock::now();

    // 动态调整块大小以适应矩阵尺寸
    size_t optimal_block_size = optimizeBlockSize(std::max(A.size(), B[0].size()));
    block_size_ = optimal_block_size;

    Matrix result = multiplyTiled(A, B);

    auto end_time = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end_time - start_time);
    metrics_.computation_time_ms = duration.count() / 1000.0;

    return result;
}

Matrix TiledMultiplier::multiplyTiled(const Matrix& A, const Matrix& B) {
    size_t rows_A = A.size();
    size_t cols_A = A[0].size();
    size_t cols_B = B[0].size();

    Matrix C = createResultMatrix(rows_A, cols_B);

    // 分块矩阵乘法 - 内存高效
    int num_threads = (optimization_level_ >= OptimizationLevel::MODERATE) ?
                     std::min(EdgeConfig::MAX_THREADS, omp_get_max_threads()) : 1;

    #pragma omp parallel for num_threads(num_threads) collapse(2) if(num_threads > 1)
    for (size_t i = 0; i < rows_A; i += block_size_) {
        for (size_t j = 0; j < cols_B; j += block_size_) {
            for (size_t k = 0; k < cols_A; k += block_size_) {
                multiplyBlock(A, B, C, i, j, block_size_);
            }
        }
    }

    return C;
}

void TiledMultiplier::multiplyBlock(const Matrix& A, const Matrix& B, Matrix& C,
                                   size_t row_start, size_t col_start, size_t block_size) {
    size_t rows_A = A.size();
    size_t cols_A = A[0].size();
    size_t cols_B = B[0].size();

    size_t i_end = std::min(row_start + block_size, rows_A);
    size_t j_end = std::min(col_start + block_size, cols_B);
    size_t k_end = std::min(row_start + block_size, cols_A);

    for (size_t i = row_start; i < i_end; ++i) {
        for (size_t j = col_start; j < j_end; ++j) {
            MatrixElement sum = C[i][j];
            #pragma omp simd reduction(+:sum) if(optimization_level_ >= OptimizationLevel::MODERATE)
            for (size_t k = row_start; k < k_end; ++k) {
                sum += A[i][k] * B[k][j];
                metrics_.operations_count += 2;
                metrics_.memory_accesses += 2;
            }
            C[i][j] = sum;
            metrics_.memory_accesses += 1;
        }
    }
}

size_t TiledMultiplier::optimizeBlockSize(size_t matrix_size) const {
    // 根据矩阵大小和可用内存动态调整块大小
    size_t cache_size_kb = 256; // 假设L2缓存256KB
    size_t element_size = sizeof(MatrixElement);

    // 计算适合缓存的块大小
    size_t optimal_block = std::sqrt(cache_size_kb * 1024 / element_size);

    // 根据矩阵大小调整
    if (matrix_size < 256) {
        optimal_block = std::min(optimal_block, size_t(16));
    } else if (matrix_size < 1024) {
        optimal_block = std::min(optimal_block, size_t(32));
    } else {
        optimal_block = std::min(optimal_block, size_t(64));
    }

    return optimal_block;
}

size_t TiledMultiplier::estimateMemoryUsage(size_t rows_A, size_t cols_A, size_t cols_B) const {
    size_t element_size = sizeof(MatrixElement);
    // 输入矩阵 + 输出矩阵 + 分块临时空间
    size_t block_memory = block_size_ * block_size_ * 3 * element_size;
    size_t matrix_memory = (rows_A * cols_A + cols_A * cols_B + rows_A * cols_B) * element_size;

    return matrix_memory + block_memory;
}

PerformanceMetrics TiledMultiplier::getPerformanceMetrics() const {
    PerformanceMetrics result = metrics_;
    // 计算缓存效率
    if (metrics_.memory_accesses > 0) {
        // 简化的缓存效率估算
        result.cache_efficiency = std::min(1.0, static_cast<double>(metrics_.operations_count) /
                                          metrics_.memory_accesses);
    }
    return result;
}

// 主矩阵乘法类实现
MatrixMultiplication::MatrixMultiplication(AlgorithmType type, OptimizationLevel level, size_t max_memory_mb)
    : algorithm_type_(type), optimization_level_(level) {
    createMultiplier();
    multiplier_->setMemoryLimit(max_memory_mb);
}

MatrixMultiplication::~MatrixMultiplication() = default;

void MatrixMultiplication::createMultiplier() {
    switch (algorithm_type_) {
        case AlgorithmType::NAIVE:
            multiplier_ = std::make_unique<NaiveMultiplier>(optimization_level_);
            break;
        case AlgorithmType::TILED:
            multiplier_ = std::make_unique<TiledMultiplier>(EdgeConfig::DEFAULT_BLOCK_SIZE, optimization_level_);
            break;
        default:
            throw std::invalid_argument("不支持的算法类型");
    }
}

Matrix MatrixMultiplication::multiply(const Matrix& A, const Matrix& B) {
    validateMatrixSize(A.size(), A[0].size(), B[0].size());

    if (!canHandleMatrix(A.size(), A[0].size(), B[0].size())) {
        throw std::runtime_error("矩阵尺寸超出内存限制");
    }

    return multiplier_->multiply(A, B);
}

size_t MatrixMultiplication::estimateMemoryUsage(size_t rows_A, size_t cols_A, size_t cols_B) const {
    return multiplier_->estimateMemoryUsage(rows_A, cols_A, cols_B);
}

bool MatrixMultiplication::canHandleMatrix(size_t rows_A, size_t cols_A, size_t cols_B) const {
    return multiplier_->checkMemoryLimit(rows_A, cols_A, cols_B);
}

void MatrixMultiplication::setAlgorithm(AlgorithmType type) {
    if (type != algorithm_type_) {
        algorithm_type_ = type;
        createMultiplier();
    }
}

void MatrixMultiplication::setOptimizationLevel(OptimizationLevel level) {
    if (level != optimization_level_) {
        optimization_level_ = level;
        createMultiplier();
    }
}

void MatrixMultiplication::setMemoryLimit(size_t max_memory_mb) {
    multiplier_->setMemoryLimit(max_memory_mb);
}

std::string MatrixMultiplication::getAlgorithmName() const {
    return multiplier_->getAlgorithmName();
}

PerformanceMetrics MatrixMultiplication::getPerformanceMetrics() const {
    return multiplier_->getPerformanceMetrics();
}

size_t MatrixMultiplication::getMemoryLimit() const {
    return multiplier_->max_memory_bytes_ / (1024 * 1024);
}

void MatrixMultiplication::validateMatrixSize(size_t rows_A, size_t cols_A, size_t cols_B) const {
    size_t max_dim = std::max({rows_A, cols_A, cols_B});
    if (max_dim > EdgeConfig::MAX_MATRIX_SIZE) {
        throw std::invalid_argument("矩阵尺寸过大: " + std::to_string(max_dim) +
                                  " > " + std::to_string(EdgeConfig::MAX_MATRIX_SIZE));
    }

    size_t total_elements = rows_A * cols_A + cols_A * cols_B + rows_A * cols_B;
    size_t max_elements = EdgeConfig::AVAILABLE_MEMORY_MB * 1024 * 1024 / sizeof(MatrixElement) / 4; // 保守估计

    if (total_elements > max_elements) {
        throw std::invalid_argument("矩阵元素总数过大: " + std::to_string(total_elements) +
                                  " > " + std::to_string(max_elements));
    }
}

} // namespace MatrixMultiplication
