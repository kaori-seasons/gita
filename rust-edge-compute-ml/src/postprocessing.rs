//! 结果后处理工具
//!
//! 提供ML模型推理结果的后处理功能，如NMS、解码等

use candle_core::Tensor;
use rust_edge_compute_core::core::error::{Result, EdgeComputeError};
use std::cmp::Ordering;

/// 检测框
#[derive(Debug, Clone)]
pub struct BoundingBox {
    /// 类别ID
    pub class_id: usize,
    /// 类别名称
    pub class_name: String,
    /// 置信度
    pub confidence: f32,
    /// 坐标 (x1, y1, x2, y2)
    pub bbox: (f32, f32, f32, f32),
}

/// 非极大值抑制（NMS）配置
#[derive(Debug, Clone)]
pub struct NMSConfig {
    /// IoU阈值
    pub iou_threshold: f32,
    /// 置信度阈值
    pub confidence_threshold: f32,
    /// 最大检测数
    pub max_detections: usize,
}

impl Default for NMSConfig {
    fn default() -> Self {
        Self {
            iou_threshold: 0.5,
            confidence_threshold: 0.5,
            max_detections: 100,
        }
    }
}

/// 后处理器
pub struct Postprocessor;

impl Postprocessor {
    /// 执行非极大值抑制（NMS）
    pub fn non_max_suppression(
        detections: Vec<BoundingBox>,
        config: &NMSConfig,
    ) -> Vec<BoundingBox> {
        // 过滤低置信度检测
        let mut filtered: Vec<BoundingBox> = detections
            .into_iter()
            .filter(|d| d.confidence >= config.confidence_threshold)
            .collect();
        
        // 按置信度排序
        filtered.sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence)
                .unwrap_or(Ordering::Equal)
        });
        
        // 执行NMS
        let mut kept = Vec::new();
        while !filtered.is_empty() && kept.len() < config.max_detections {
            // 取置信度最高的
            let best = filtered.remove(0);
            kept.push(best.clone());
            
            // 移除与best IoU过高的检测
            filtered.retain(|det| {
                let iou = Self::calculate_iou(&best.bbox, &det.bbox);
                iou < config.iou_threshold
            });
        }
        
        kept
    }
    
    /// 计算IoU（Intersection over Union）
    fn calculate_iou(bbox1: &(f32, f32, f32, f32), bbox2: &(f32, f32, f32, f32)) -> f32 {
        let (x1_1, y1_1, x2_1, y2_1) = bbox1;
        let (x1_2, y1_2, x2_2, y2_2) = bbox2;
        
        // 计算交集
        let x1_i = x1_1.max(*x1_2);
        let y1_i = y1_1.max(*y1_2);
        let x2_i = x2_1.min(*x2_2);
        let y2_i = y2_1.min(*y2_2);
        
        if x2_i < x1_i || y2_i < y1_i {
            return 0.0;
        }
        
        let intersection = (x2_i - x1_i) * (y2_i - y1_i);
        
        // 计算并集
        let area1 = (x2_1 - x1_1) * (y2_1 - y1_1);
        let area2 = (x2_2 - x1_2) * (y2_2 - y1_2);
        let union = area1 + area2 - intersection;
        
        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }
    
    /// 从Tensor提取分类结果
    pub fn extract_classifications(
        tensor: &Tensor,
        class_names: &[String],
        top_k: Option<usize>,
    ) -> Result<Vec<(String, f32)>> {
        // 获取logits（假设是 [batch, classes] 形状）
        let shape = tensor.shape();
        if shape.dims().len() != 2 {
            return Err(EdgeComputeError::AlgorithmExecution {
                message: format!("Expected 2D tensor, got shape: {:?}", shape),
                algorithm: Some("classification_extraction".to_string()),
                input_size: None,
            });
        }
        
        let batch_size = shape.dims()[0];
        let num_classes = shape.dims()[1];
        
        // 取第一个batch的结果
        let logits = tensor.to_vec2::<f32>()?;
        if batch_size == 0 {
            return Ok(Vec::new());
        }
        
        let scores = &logits[0];
        
        // 创建(类别索引, 分数)对
        let mut class_scores: Vec<(usize, f32)> = scores
            .iter()
            .enumerate()
            .map(|(i, &score)| (i, score))
            .collect();
        
        // 按分数排序
        class_scores.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
        });
        
        // 取top_k
        let k = top_k.unwrap_or(num_classes);
        let top_k_scores = class_scores.into_iter().take(k);
        
        // 转换为(类别名, 分数)对
        let mut results = Vec::new();
        for (class_idx, score) in top_k_scores {
            let class_name = if class_idx < class_names.len() {
                class_names[class_idx].clone()
            } else {
                format!("class_{}", class_idx)
            };
            results.push((class_name, score));
        }
        
        Ok(results)
    }
    
    /// 应用softmax
    pub fn softmax(tensor: &Tensor) -> Result<Tensor> {
        // 计算exp
        let exp = tensor.exp()?;
        
        // 计算sum
        let sum = exp.sum_keepdim(1)?;
        
        // 归一化
        exp.broadcast_div(&sum)
            .map_err(|e| EdgeComputeError::AlgorithmExecution {
                message: format!("Failed to apply softmax: {}", e),
                algorithm: Some("softmax".to_string()),
                input_size: None,
            })
    }
    
    /// 从文本生成结果解码
    pub fn decode_text_generation(
        token_ids: &Tensor,
        vocab: &[String],
    ) -> Result<String> {
        // 获取token IDs
        let ids = token_ids.to_vec1::<u32>()?;
        
        // 转换为文本
        let tokens: Vec<String> = ids
            .iter()
            .filter_map(|&id| {
                if (id as usize) < vocab.len() {
                    Some(vocab[id as usize].clone())
                } else {
                    None
                }
            })
            .collect();
        
        Ok(tokens.join(""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_nms_config() {
        let config = NMSConfig::default();
        assert_eq!(config.iou_threshold, 0.5);
        assert_eq!(config.confidence_threshold, 0.5);
    }
    
    #[test]
    fn test_calculate_iou() {
        let bbox1 = (0.0, 0.0, 10.0, 10.0);
        let bbox2 = (5.0, 5.0, 15.0, 15.0);
        let iou = Postprocessor::calculate_iou(&bbox1, &bbox2);
        assert!(iou > 0.0 && iou <= 1.0);
    }
    
    #[test]
    fn test_nms() {
        let detections = vec![
            BoundingBox {
                class_id: 0,
                class_name: "class1".to_string(),
                confidence: 0.9,
                bbox: (0.0, 0.0, 10.0, 10.0),
            },
            BoundingBox {
                class_id: 0,
                class_name: "class1".to_string(),
                confidence: 0.8,
                bbox: (1.0, 1.0, 11.0, 11.0),
            },
        ];
        
        let config = NMSConfig::default();
        let result = Postprocessor::non_max_suppression(detections, &config);
        assert!(!result.is_empty());
    }
}

