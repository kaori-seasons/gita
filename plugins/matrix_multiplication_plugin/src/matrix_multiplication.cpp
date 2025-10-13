// 矩阵乘法算法实现文件
// 生产级实现，支持多种算法和优化策略

#include "matrix_multiplication.hpp"
#include <algorithm>
#include <cmath>
#include <cstring>
#include <stdexcept>
#include <iostream>
#include <omp.h>

namespace MatrixMultiplication {

// 验证矩阵维度
void MatrixMultiplier::validateMatrices(const Matrix& A, const Matrix& B) {
    if (A.empty() || B.empty()) {
        throw std::invalid_argument("输入矩阵不能为空");
    }

    size_t cols_A = A[0].size();
    size_t rows_B = B.size();

    // 检查矩阵A的每一行长度是否一致
    for (const auto& row : A) {
        if (row.size() != cols_A) {
            throw std::invalid_argument("矩阵A的行长度不一致");
        }
    }

    // 检查矩阵B的每一行长度是否一致
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
}

// 创建结果矩阵
Matrix MatrixMultiplier::createResultMatrix(size_t rows, size_t cols, MatrixElement initial_value) {
    return Matrix(rows, MatrixRow(cols, initial_value));
}

// 朴素矩阵乘法实现
NaiveMultiplier::NaiveMultiplier(OptimizationLevel level)
    : optimization_level_(level), metrics_() {}

Matrix NaiveMultiplier::multiply(const Matrix& A, const Matrix& B) {
    validateMatrices(A, B);

    size_t rows_A = A.size();
    size_t cols_A = A[0].size();
    size_t cols_B = B[0].size();

    Matrix C = createResultMatrix(rows_A, cols_B);

    metrics_.operations_count = 0;
    metrics_.memory_accesses = 0;

    // 根据优化级别选择不同的实现
    if (optimization_level_ >= OptimizationLevel::BASIC) {
        // 基础优化：循环展开和更好的缓存利用
        #pragma omp parallel for collapse(2) if(optimization_level_ >= OptimizationLevel::ADVANCED)
        for (size_t i = 0; i < rows_A; ++i) {
            for (size_t j = 0; j < cols_B; ++j) {
                MatrixElement sum = 0.0;
                #pragma omp simd reduction(+:sum) if(optimization_level_ >= OptimizationLevel::AGGRESSIVE)
                for (size_t k = 0; k < cols_A; ++k) {
                    sum += A[i][k] * B[k][j];
                    metrics_.operations_count += 2; // 乘法和加法
                    metrics_.memory_accesses += 2; // 读取A[i][k]和B[k][j]
                }
                C[i][j] = sum;
                metrics_.memory_accesses += 1; // 写入C[i][j]
            }
        }
    } else {
        // 基础实现
        for (size_t i = 0; i < rows_A; ++i) {
            for (size_t j = 0; j < cols_B; ++j) {
                MatrixElement sum = 0.0;
                for (size_t k = 0; k < cols_A; ++k) {
                    sum += A[i][k] * B[k][j];
                    metrics_.operations_count += 2;
                    metrics_.memory_accesses += 2;
                }
                C[i][j] = sum;
                metrics_.memory_accesses += 1;
            }
        }
    }

    // 计算计算强度
    size_t total_elements = rows_A * cols_B * cols_A;
    metrics_.computation_intensity = static_cast<double>(metrics_.operations_count) / total_elements;
    metrics_.arithmetic_intensity = static_cast<double>(metrics_.operations_count) / metrics_.memory_accesses;

    return C;
}

size_t NaiveMultiplier::estimateMemoryUsage(size_t rows_A, size_t cols_B) const {
    // 估算内存使用：输入矩阵 + 输出矩阵 + 临时变量
    size_t element_size = sizeof(MatrixElement);
    return (rows_A * cols_B + rows_A * cols_B) * element_size * 2; // 保守估算
}

PerformanceMetrics NaiveMultiplier::getPerformanceMetrics() const {
    return metrics_;
}

// 分块矩阵乘法实现
TiledMultiplier::TiledMultiplier(size_t block_size, OptimizationLevel level)
    : block_size_(block_size), optimization_level_(level), metrics_() {}

Matrix TiledMultiplier::multiply(const Matrix& A, const Matrix& B) {
    validateMatrices(A, B);

    if (optimization_level_ >= OptimizationLevel::ADVANCED) {
        return multiplyTiled(A, B);
    } else {
        // 如果优化级别不够，使用朴素算法
        NaiveMultiplier naive(optimization_level_);
        return naive.multiply(A, B);
    }
}

Matrix TiledMultiplier::multiplyTiled(const Matrix& A, const Matrix& B) {
    size_t rows_A = A.size();
    size_t cols_A = A[0].size();
    size_t cols_B = B[0].size();

    Matrix C = createResultMatrix(rows_A, cols_B);

    metrics_.operations_count = 0;
    metrics_.memory_accesses = 0;

    // 分块矩阵乘法
    #pragma omp parallel for collapse(2)
    for (size_t i = 0; i < rows_A; i += block_size_) {
        for (size_t j = 0; j < cols_B; j += block_size_) {
            for (size_t k = 0; k < cols_A; k += block_size_) {
                multiplyBlock(A, B, C, i, j, block_size_);
            }
        }
    }

    size_t total_elements = rows_A * cols_B * cols_A;
    metrics_.computation_intensity = static_cast<double>(metrics_.operations_count) / total_elements;
    metrics_.arithmetic_intensity = static_cast<double>(metrics_.operations_count) / metrics_.memory_accesses;

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
            MatrixElement sum = C[i][j]; // 使用现有值
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

size_t TiledMultiplier::estimateMemoryUsage(size_t rows_A, size_t cols_B) const {
    size_t element_size = sizeof(MatrixElement);
    size_t block_elements = block_size_ * block_size_;
    // 分块算法需要额外的块缓存
    return (rows_A * cols_B + block_elements * 3) * element_size;
}

PerformanceMetrics TiledMultiplier::getPerformanceMetrics() const {
    return metrics_;
}

// Strassen算法实现
StrassenMultiplier::StrassenMultiplier(size_t threshold, OptimizationLevel level)
    : threshold_(threshold), optimization_level_(level), metrics_() {}

Matrix StrassenMultiplier::multiply(const Matrix& A, const Matrix& B) {
    validateMatrices(A, B);

    if (A.size() <= threshold_ || A[0].size() <= threshold_ || B[0].size() <= threshold_) {
        // 对于小矩阵，使用朴素算法
        NaiveMultiplier naive(optimization_level_);
        return naive.multiply(A, B);
    }

    return strassenMultiply(A, B);
}

Matrix StrassenMultiplier::strassenMultiply(const Matrix& A, const Matrix& B) {
    size_t n = A.size();
    size_t m = A[0].size();
    size_t p = B[0].size();

    // 扩展矩阵到2^k大小
    size_t new_size = 1;
    while (new_size < std::max({n, m, p})) {
        new_size *= 2;
    }

    // 如果矩阵已经是2^k大小，直接计算，否则需要填充
    if (n == new_size && m == new_size && p == new_size) {
        return strassenMultiplyRecursive(A, B);
    } else {
        // 扩展矩阵（这里简化为使用朴素算法）
        NaiveMultiplier naive(optimization_level_);
        return naive.multiply(A, B);
    }
}

Matrix StrassenMultiplier::strassenMultiplyRecursive(const Matrix& A, const Matrix& B) {
    size_t n = A.size();

    if (n <= 1) {
        Matrix C(1, MatrixRow(1, A[0][0] * B[0][0]));
        metrics_.operations_count += 1;
        metrics_.memory_accesses += 3;
        return C;
    }

    size_t half = n / 2;

    // 分割矩阵
    Matrix A11, A12, A21, A22;
    Matrix B11, B12, B21, B22;

    splitMatrix(A, A11, A12, A21, A22);
    splitMatrix(B, B11, B12, B21, B22);

    // 计算7个子矩阵乘法
    Matrix P1 = strassenMultiplyRecursive(A11, subtractMatrices(B12, B22));
    Matrix P2 = strassenMultiplyRecursive(addMatrices(A11, A12), B22);
    Matrix P3 = strassenMultiplyRecursive(addMatrices(A21, A22), B11);
    Matrix P4 = strassenMultiplyRecursive(A22, subtractMatrices(B21, B11));
    Matrix P5 = strassenMultiplyRecursive(addMatrices(A11, A22), addMatrices(B11, B22));
    Matrix P6 = strassenMultiplyRecursive(subtractMatrices(A12, A22), addMatrices(B21, B22));
    Matrix P7 = strassenMultiplyRecursive(subtractMatrices(A11, A21), addMatrices(B11, B12));

    // 计算结果矩阵块
    Matrix C11 = addMatrices(subtractMatrices(addMatrices(P5, P4), P2), P6);
    Matrix C12 = addMatrices(P1, P2);
    Matrix C21 = addMatrices(P3, P4);
    Matrix C22 = subtractMatrices(subtractMatrices(addMatrices(P5, P1), P3), P7);

    // 组合结果矩阵
    return combineMatrices(C11, C12, C21, C22);
}

Matrix StrassenMultiplier::addMatrices(const Matrix& A, const Matrix& B) {
    size_t n = A.size();
    size_t m = A[0].size();
    Matrix C = createResultMatrix(n, m);

    for (size_t i = 0; i < n; ++i) {
        for (size_t j = 0; j < m; ++j) {
            C[i][j] = A[i][j] + B[i][j];
        }
    }
    return C;
}

Matrix StrassenMultiplier::subtractMatrices(const Matrix& A, const Matrix& B) {
    size_t n = A.size();
    size_t m = A[0].size();
    Matrix C = createResultMatrix(n, m);

    for (size_t i = 0; i < n; ++i) {
        for (size_t j = 0; j < m; ++j) {
            C[i][j] = A[i][j] - B[i][j];
        }
    }
    return C;
}

void StrassenMultiplier::splitMatrix(const Matrix& source,
                                    Matrix& A11, Matrix& A12, Matrix& A21, Matrix& A22) {
    size_t n = source.size();
    size_t half = n / 2;

    A11.resize(half, MatrixRow(half));
    A12.resize(half, MatrixRow(half));
    A21.resize(half, MatrixRow(half));
    A22.resize(half, MatrixRow(half));

    for (size_t i = 0; i < half; ++i) {
        for (size_t j = 0; j < half; ++j) {
            A11[i][j] = source[i][j];
            A12[i][j] = source[i][j + half];
            A21[i][j] = source[i + half][j];
            A22[i][j] = source[i + half][j + half];
        }
    }
}

Matrix StrassenMultiplier::combineMatrices(const Matrix& A11, const Matrix& A12,
                                          const Matrix& A21, const Matrix& A22) {
    size_t half = A11.size();
    size_t n = half * 2;
    Matrix C = createResultMatrix(n, n);

    for (size_t i = 0; i < half; ++i) {
        for (size_t j = 0; j < half; ++j) {
            C[i][j] = A11[i][j];
            C[i][j + half] = A12[i][j];
            C[i + half][j] = A21[i][j];
            C[i + half][j + half] = A22[i][j];
        }
    }
    return C;
}

size_t StrassenMultiplier::estimateMemoryUsage(size_t rows_A, size_t cols_B) const {
    size_t element_size = sizeof(MatrixElement);
    // Strassen算法需要更多的临时空间
    return (rows_A * cols_B * 7) * element_size;
}

PerformanceMetrics StrassenMultiplier::getPerformanceMetrics() const {
    return metrics_;
}

#ifdef USE_EIGEN
// Eigen库实现
EigenMultiplier::EigenMultiplier(OptimizationLevel level)
    : optimization_level_(level), metrics_() {}

Matrix EigenMultiplier::multiply(const Matrix& A, const Matrix& B) {
    validateMatrices(A, B);

    size_t rows_A = A.size();
    size_t cols_A = A[0].size();
    size_t cols_B = B[0].size();

    // 转换为Eigen矩阵
    Eigen::MatrixXd eigen_A(rows_A, cols_A);
    Eigen::MatrixXd eigen_B(cols_A, cols_B);

    for (size_t i = 0; i < rows_A; ++i) {
        for (size_t j = 0; j < cols_A; ++j) {
            eigen_A(i, j) = A[i][j];
        }
    }

    for (size_t i = 0; i < cols_A; ++i) {
        for (size_t j = 0; j < cols_B; ++j) {
            eigen_B(i, j) = B[i][j];
        }
    }

    // 执行矩阵乘法
    auto start_time = std::chrono::high_resolution_clock::now();
    Eigen::MatrixXd eigen_C = eigen_A * eigen_B;
    auto end_time = std::chrono::high_resolution_clock::now();

    // 转换回标准矩阵
    Matrix C = createResultMatrix(rows_A, cols_B);
    for (size_t i = 0; i < rows_A; ++i) {
        for (size_t j = 0; j < cols_B; ++j) {
            C[i][j] = eigen_C(i, j);
        }
    }

    // 更新性能指标
    metrics_.operations_count = rows_A * cols_A * cols_B * 2; // 估算值
    metrics_.memory_accesses = (rows_A * cols_A + cols_A * cols_B + rows_A * cols_B) * 2;
    metrics_.computation_intensity = static_cast<double>(metrics_.operations_count) /
                                    (rows_A * cols_B * cols_A);
    metrics_.arithmetic_intensity = static_cast<double>(metrics_.operations_count) /
                                   metrics_.memory_accesses;

    return C;
}

size_t EigenMultiplier::estimateMemoryUsage(size_t rows_A, size_t cols_B) const {
    size_t element_size = sizeof(MatrixElement);
    return (rows_A * cols_B * 3) * element_size; // 输入+输出+临时空间
}

PerformanceMetrics EigenMultiplier::getPerformanceMetrics() const {
    return metrics_;
}
#endif

#ifdef USE_OPENBLAS
// OpenBLAS实现
OpenBLASMultiplier::OpenBLASMultiplier(OptimizationLevel level)
    : optimization_level_(level), metrics_() {}

Matrix OpenBLASMultiplier::multiply(const Matrix& A, const Matrix& B) {
    validateMatrices(A, B);

    size_t rows_A = A.size();
    size_t cols_A = A[0].size();
    size_t cols_B = B[0].size();

    Matrix C = createResultMatrix(rows_A, cols_B);

    // 转换为连续内存布局
    std::vector<double> A_flat(rows_A * cols_A);
    std::vector<double> B_flat(cols_A * cols_B);
    std::vector<double> C_flat(rows_A * cols_B);

    // 填充数据 (行优先)
    for (size_t i = 0; i < rows_A; ++i) {
        for (size_t j = 0; j < cols_A; ++j) {
            A_flat[i * cols_A + j] = A[i][j];
        }
    }

    for (size_t i = 0; i < cols_A; ++i) {
        for (size_t j = 0; j < cols_B; ++j) {
            B_flat[i * cols_B + j] = B[i][j];
        }
    }

    // 调用OpenBLAS dgemm
    cblas_dgemm(CblasRowMajor, CblasNoTrans, CblasNoTrans,
                rows_A, cols_B, cols_A,
                1.0, A_flat.data(), cols_A,
                B_flat.data(), cols_B,
                0.0, C_flat.data(), cols_B);

    // 转换回矩阵格式
    for (size_t i = 0; i < rows_A; ++i) {
        for (size_t j = 0; j < cols_B; ++j) {
            C[i][j] = C_flat[i * cols_B + j];
        }
    }

    // 更新性能指标
    metrics_.operations_count = rows_A * cols_A * cols_B * 2;
    metrics_.memory_accesses = (rows_A * cols_A + cols_A * cols_B + rows_A * cols_B) * 2;
    metrics_.computation_intensity = static_cast<double>(metrics_.operations_count) /
                                    (rows_A * cols_B * cols_A);
    metrics_.arithmetic_intensity = static_cast<double>(metrics_.operations_count) /
                                   metrics_.memory_accesses;

    return C;
}

size_t OpenBLASMultiplier::estimateMemoryUsage(size_t rows_A, size_t cols_B) const {
    size_t element_size = sizeof(MatrixElement);
    return (rows_A * cols_B * 3) * element_size;
}

PerformanceMetrics OpenBLASMultiplier::getPerformanceMetrics() const {
    return metrics_;
}
#endif

// 主矩阵乘法类实现
MatrixMultiplication::MatrixMultiplication(AlgorithmType type, int optimization_level)
    : algorithm_type_(type), optimization_level_(static_cast<OptimizationLevel>(optimization_level)) {
    createMultiplier();
}

MatrixMultiplication::~MatrixMultiplication() = default;

void MatrixMultiplication::createMultiplier() {
    switch (algorithm_type_) {
        case AlgorithmType::NAIVE:
            multiplier_ = std::make_unique<NaiveMultiplier>(optimization_level_);
            break;
        case AlgorithmType::TILED:
            multiplier_ = std::make_unique<TiledMultiplier>(64, optimization_level_);
            break;
        case AlgorithmType::STRASSEN:
            multiplier_ = std::make_unique<StrassenMultiplier>(128, optimization_level_);
            break;
#ifdef USE_EIGEN
        case AlgorithmType::EIGEN:
            multiplier_ = std::make_unique<EigenMultiplier>(optimization_level_);
            break;
#endif
#ifdef USE_OPENBLAS
        case AlgorithmType::OPENBLAS:
            multiplier_ = std::make_unique<OpenBLASMultiplier>(optimization_level_);
            break;
#endif
        default:
            throw std::invalid_argument("不支持的算法类型");
    }
}

Matrix MatrixMultiplication::multiply(const Matrix& A, const Matrix& B) {
    if (!multiplier_) {
        throw std::runtime_error("矩阵乘法器未初始化");
    }
    return multiplier_->multiply(A, B);
}

size_t MatrixMultiplication::estimateMemoryUsage(size_t rows_A, size_t cols_B) const {
    if (!multiplier_) {
        return 0;
    }
    return multiplier_->estimateMemoryUsage(rows_A, cols_B);
}

std::string MatrixMultiplication::getAlgorithmName() const {
    if (!multiplier_) {
        return "Unknown";
    }
    return multiplier_->getAlgorithmName();
}

PerformanceMetrics MatrixMultiplication::getPerformanceMetrics() const {
    if (!multiplier_) {
        return PerformanceMetrics{};
    }
    return multiplier_->getPerformanceMetrics();
}

} // namespace MatrixMultiplication
