// 边缘端优化的矩阵乘法算法头文件
// 针对8GB内存、4核CPU的资源受限环境优化

#ifndef MATRIX_MULTIPLICATION_EDGE_OPTIMIZED_HPP
#define MATRIX_MULTIPLICATION_EDGE_OPTIMIZED_HPP

#include <vector>
#include <memory>
#include <string>
#include <stdexcept>

namespace MatrixMultiplication {

// 边缘端优化配置
struct EdgeConfig {
    static constexpr size_t TOTAL_MEMORY_MB = 8192;     // 总内存8GB
    static constexpr size_t AVAILABLE_MEMORY_MB = 4096; // 可用内存4GB (保守估计)
    static constexpr int CPU_CORES = 4;                 // CPU核心数
    static constexpr int MAX_THREADS = 2;               // 最大线程数 (避免过度并行)
    static constexpr size_t MAX_MATRIX_SIZE = 2048;     // 最大矩阵尺寸
    static constexpr size_t DEFAULT_BLOCK_SIZE = 32;    // 默认分块大小
};

// 类型定义 - 内存优化
using MatrixElement = float;  // 使用float节省50%的内存
using Matrix = std::vector<std::vector<MatrixElement>>;
using MatrixRow = std::vector<MatrixElement>;

// 内存池 - 减少内存分配开销
class MemoryPool {
public:
    explicit MemoryPool(size_t initial_capacity = 1024 * 1024); // 1M elements
    ~MemoryPool() = default;

    // 获取内存块
    MatrixElement* allocate(size_t size);
    void deallocate(MatrixElement* ptr, size_t size);

    // 内存池状态
    size_t used() const { return used_; }
    size_t capacity() const { return pool_.size(); }
    void reset();

private:
    std::vector<MatrixElement> pool_;
    size_t used_;
    std::vector<std::pair<size_t, size_t>> free_blocks_; // (offset, size)
};

// 算法类型枚举 - 只保留内存高效的算法
enum class AlgorithmType {
    NAIVE,      // 朴素算法 - 内存最省
    TILED,      // 分块算法 - 缓存友好
};

// 优化级别 - 针对边缘端调整
enum class OptimizationLevel {
    NONE = 0,       // 无优化 - 最低内存使用
    BASIC = 1,      // 基础优化 - 平衡性能和内存
    MODERATE = 2,   // 中等优化 - 适度性能提升
};

// 性能指标结构体
struct PerformanceMetrics {
    size_t operations_count = 0;
    size_t memory_accesses = 0;
    double computation_time_ms = 0.0;
    size_t peak_memory_usage = 0;
    double cache_efficiency = 0.0;
};

// 矩阵乘法器基类 - 边缘端优化
class MatrixMultiplier {
public:
    explicit MatrixMultiplier(size_t max_memory_mb = EdgeConfig::AVAILABLE_MEMORY_MB / 4);
    virtual ~MatrixMultiplier() = default;

    // 核心方法
    virtual Matrix multiply(const Matrix& A, const Matrix& B) = 0;
    virtual size_t estimateMemoryUsage(size_t rows_A, size_t cols_A, size_t cols_B) const = 0;
    virtual PerformanceMetrics getPerformanceMetrics() const = 0;
    virtual std::string getAlgorithmName() const = 0;

    // 内存管理
    bool checkMemoryLimit(size_t rows_A, size_t cols_A, size_t cols_B) const;
    void setMemoryLimit(size_t max_memory_mb);

protected:
    size_t max_memory_bytes_;
    mutable MemoryPool memory_pool_;
    mutable PerformanceMetrics metrics_;

    // 工具方法
    static void validateMatrices(const Matrix& A, const Matrix& B);
    Matrix createResultMatrix(size_t rows, size_t cols) const;
    void updateMemoryUsage(size_t additional_bytes) const;
    void resetMetrics();

    // 内存池辅助方法
    void preallocateForOperation(size_t rows_A, size_t cols_A, size_t cols_B);
};

// 朴素矩阵乘法实现 - 边缘端优化
class NaiveMultiplier : public MatrixMultiplier {
public:
    explicit NaiveMultiplier(OptimizationLevel level = OptimizationLevel::BASIC);
    ~NaiveMultiplier() override = default;

    Matrix multiply(const Matrix& A, const Matrix& B) override;
    size_t estimateMemoryUsage(size_t rows_A, size_t cols_A, size_t cols_B) const override;
    PerformanceMetrics getPerformanceMetrics() const override;
    std::string getAlgorithmName() const override { return "Naive"; }

private:
    OptimizationLevel optimization_level_;

    Matrix multiplyBasic(const Matrix& A, const Matrix& B);
    Matrix multiplyOptimized(const Matrix& A, const Matrix& B);
};

// 分块矩阵乘法实现 - 边缘端优化
class TiledMultiplier : public MatrixMultiplier {
public:
    explicit TiledMultiplier(size_t block_size = EdgeConfig::DEFAULT_BLOCK_SIZE,
                           OptimizationLevel level = OptimizationLevel::MODERATE);
    ~TiledMultiplier() override = default;

    Matrix multiply(const Matrix& A, const Matrix& B) override;
    size_t estimateMemoryUsage(size_t rows_A, size_t cols_A, size_t cols_B) const override;
    PerformanceMetrics getPerformanceMetrics() const override;
    std::string getAlgorithmName() const override { return "Tiled"; }

private:
    size_t block_size_;
    OptimizationLevel optimization_level_;

    Matrix multiplyTiled(const Matrix& A, const Matrix& B);
    void multiplyBlock(const Matrix& A, const Matrix& B, Matrix& C,
                      size_t row_start, size_t col_start, size_t block_size);
    size_t optimizeBlockSize(size_t matrix_size) const;
};

// 主矩阵乘法类 - 边缘端版本
class MatrixMultiplication {
public:
    explicit MatrixMultiplication(AlgorithmType type = AlgorithmType::TILED,
                                  OptimizationLevel level = OptimizationLevel::BASIC,
                                  size_t max_memory_mb = EdgeConfig::AVAILABLE_MEMORY_MB / 4);
    ~MatrixMultiplication();

    // 执行矩阵乘法
    Matrix multiply(const Matrix& A, const Matrix& B);

    // 资源管理
    size_t estimateMemoryUsage(size_t rows_A, size_t cols_A, size_t cols_B) const;
    bool canHandleMatrix(size_t rows_A, size_t cols_A, size_t cols_B) const;

    // 配置管理
    void setAlgorithm(AlgorithmType type);
    void setOptimizationLevel(OptimizationLevel level);
    void setMemoryLimit(size_t max_memory_mb);

    // 信息查询
    AlgorithmType getAlgorithmType() const { return algorithm_type_; }
    std::string getAlgorithmName() const;
    PerformanceMetrics getPerformanceMetrics() const;
    size_t getMemoryLimit() const;

private:
    AlgorithmType algorithm_type_;
    OptimizationLevel optimization_level_;
    std::unique_ptr<MatrixMultiplier> multiplier_;

    void createMultiplier();
    void validateMatrixSize(size_t rows_A, size_t cols_A, size_t cols_B) const;
};

} // namespace MatrixMultiplication

#endif // MATRIX_MULTIPLICATION_EDGE_OPTIMIZED_HPP
