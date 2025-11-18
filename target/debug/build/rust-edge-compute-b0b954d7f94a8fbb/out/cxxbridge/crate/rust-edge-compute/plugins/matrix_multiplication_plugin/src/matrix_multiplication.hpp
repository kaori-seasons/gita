// 矩阵乘法算法头文件
// 生产级实现，支持多种算法和优化策略

#ifndef MATRIX_MULTIPLICATION_HPP
#define MATRIX_MULTIPLICATION_HPP

#include <vector>
#include <memory>
#include <functional>
#include <stdexcept>

#ifdef USE_EIGEN
#include <Eigen/Dense>
#endif

#ifdef USE_OPENBLAS
extern "C" {
#include <cblas.h>
}
#endif

namespace MatrixMultiplication {

// 类型定义 - 针对边缘端内存优化
using MatrixElement = float;  // 使用float而不是double节省内存
using Matrix = std::vector<std::vector<MatrixElement>>;
using MatrixRow = std::vector<MatrixElement>;

// 内存池类型 - 减少内存分配开销
using MemoryPool = std::vector<MatrixElement>;

// 算法类型枚举 - 针对边缘端优化
enum class AlgorithmType {
    NAIVE,      // 朴素算法 O(n^3) - 内存高效
    TILED,      // 分块算法 - 缓存优化，内存友好
    // 移除了Strassen算法（内存消耗大，不适合边缘端）
    // 移除了Eigen和OpenBLAS（依赖复杂，不适合资源受限环境）
};

// 优化级别
enum class OptimizationLevel {
    NONE = 0,       // 无优化
    BASIC = 1,      // 基础优化
    ADVANCED = 2,   // 高级优化
    AGGRESSIVE = 3  // 激进优化
};

// 性能指标结构体
struct PerformanceMetrics {
    size_t operations_count = 0;      // 操作次数
    size_t memory_accesses = 0;       // 内存访问次数
    size_t cache_misses = 0;          // 缓存未命中次数
    double computation_intensity = 0.0; // 计算强度
    double arithmetic_intensity = 0.0;  // 算术强度
};

// 矩阵乘法器类 - 边缘端优化版本
class MatrixMultiplier {
public:
    explicit MatrixMultiplier(size_t max_memory_mb = 1024);  // 默认1GB内存限制
    virtual ~MatrixMultiplier() = default;

    // 执行矩阵乘法 - 带内存限制检查
    virtual Matrix multiply(const Matrix& A, const Matrix& B) = 0;

    // 估算内存使用量
    virtual size_t estimateMemoryUsage(size_t rows_A, size_t cols_B) const = 0;

    // 获取性能指标
    virtual PerformanceMetrics getPerformanceMetrics() const = 0;

    // 获取算法名称
    virtual std::string getAlgorithmName() const = 0;

    // 检查内存限制
    bool checkMemoryLimit(size_t rows_A, size_t cols_A, size_t cols_B) const;

protected:
    size_t max_memory_bytes_;  // 最大内存使用量（字节）
    mutable MemoryPool memory_pool_;  // 内存池减少分配开销

    // 验证矩阵维度
    static void validateMatrices(const Matrix& A, const Matrix& B);

    // 创建结果矩阵 - 使用内存池优化
    Matrix createResultMatrix(size_t rows, size_t cols, MatrixElement initial_value = 0.0f);

    // 预分配内存池
    void preallocateMemoryPool(size_t size);
};

// 朴素矩阵乘法实现
class NaiveMultiplier : public MatrixMultiplier {
public:
    explicit NaiveMultiplier(OptimizationLevel level = OptimizationLevel::BASIC);
    ~NaiveMultiplier() override = default;

    Matrix multiply(const Matrix& A, const Matrix& B) override;
    size_t estimateMemoryUsage(size_t rows_A, size_t cols_B) const override;
    PerformanceMetrics getPerformanceMetrics() const override;
    std::string getAlgorithmName() const override { return "Naive"; }

private:
    OptimizationLevel optimization_level_;
    mutable PerformanceMetrics metrics_;
};

// 分块矩阵乘法实现
class TiledMultiplier : public MatrixMultiplier {
public:
    explicit TiledMultiplier(size_t block_size = 64, OptimizationLevel level = OptimizationLevel::ADVANCED);
    ~TiledMultiplier() override = default;

    Matrix multiply(const Matrix& A, const Matrix& B) override;
    size_t estimateMemoryUsage(size_t rows_A, size_t cols_B) const override;
    PerformanceMetrics getPerformanceMetrics() const override;
    std::string getAlgorithmName() const override { return "Tiled"; }

private:
    size_t block_size_;
    OptimizationLevel optimization_level_;
    mutable PerformanceMetrics metrics_;

    Matrix multiplyTiled(const Matrix& A, const Matrix& B);
    void multiplyBlock(const Matrix& A, const Matrix& B, Matrix& C,
                      size_t row_start, size_t col_start, size_t block_size);
};

// Strassen算法实现
class StrassenMultiplier : public MatrixMultiplier {
public:
    explicit StrassenMultiplier(size_t threshold = 128, OptimizationLevel level = OptimizationLevel::AGGRESSIVE);
    ~StrassenMultiplier() override = default;

    Matrix multiply(const Matrix& A, const Matrix& B) override;
    size_t estimateMemoryUsage(size_t rows_A, size_t cols_B) const override;
    PerformanceMetrics getPerformanceMetrics() const override;
    std::string getAlgorithmName() const override { return "Strassen"; }

private:
    size_t threshold_;
    OptimizationLevel optimization_level_;
    mutable PerformanceMetrics metrics_;

    Matrix strassenMultiply(const Matrix& A, const Matrix& B);
    Matrix addMatrices(const Matrix& A, const Matrix& B);
    Matrix subtractMatrices(const Matrix& A, const Matrix& B);
    void splitMatrix(const Matrix& source, Matrix& A11, Matrix& A12, Matrix& A21, Matrix& A22);
    Matrix combineMatrices(const Matrix& A11, const Matrix& A12, const Matrix& A21, const Matrix& A22);
};

#ifdef USE_EIGEN
// Eigen库实现
class EigenMultiplier : public MatrixMultiplier {
public:
    explicit EigenMultiplier(OptimizationLevel level = OptimizationLevel::ADVANCED);
    ~EigenMultiplier() override = default;

    Matrix multiply(const Matrix& A, const Matrix& B) override;
    size_t estimateMemoryUsage(size_t rows_A, size_t cols_B) const override;
    PerformanceMetrics getPerformanceMetrics() const override;
    std::string getAlgorithmName() const override { return "Eigen"; }

private:
    OptimizationLevel optimization_level_;
    mutable PerformanceMetrics metrics_;
};
#endif

#ifdef USE_OPENBLAS
// OpenBLAS实现
class OpenBLASMultiplier : public MatrixMultiplier {
public:
    explicit OpenBLASMultiplier(OptimizationLevel level = OptimizationLevel::AGGRESSIVE);
    ~OpenBLASMultiplier() override = default;

    Matrix multiply(const Matrix& A, const Matrix& B) override;
    size_t estimateMemoryUsage(size_t rows_A, size_t cols_B) const override;
    PerformanceMetrics getPerformanceMetrics() const override;
    std::string getAlgorithmName() const override { return "OpenBLAS"; }

private:
    OptimizationLevel optimization_level_;
    mutable PerformanceMetrics metrics_;
};
#endif

// 主矩阵乘法类
class MatrixMultiplication {
public:
    explicit MatrixMultiplication(AlgorithmType type = AlgorithmType::NAIVE,
                                  int optimization_level = 1);
    ~MatrixMultiplication();

    // 执行矩阵乘法
    Matrix multiply(const Matrix& A, const Matrix& B);

    // 估算内存使用量
    size_t estimateMemoryUsage(size_t rows_A, size_t cols_B) const;

    // 获取当前算法
    AlgorithmType getAlgorithmType() const { return algorithm_type_; }

    // 获取算法名称
    std::string getAlgorithmName() const;

    // 获取性能指标
    PerformanceMetrics getPerformanceMetrics() const;

private:
    AlgorithmType algorithm_type_;
    OptimizationLevel optimization_level_;
    std::unique_ptr<MatrixMultiplier> multiplier_;

    void createMultiplier();
};

} // namespace MatrixMultiplication

#endif // MATRIX_MULTIPLICATION_HPP
