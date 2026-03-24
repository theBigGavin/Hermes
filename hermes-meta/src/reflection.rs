//! 反思系统 - 深度自我分析

use std::sync::Arc;

use hermes_core::{Evaluation, Timestamp, now};
use hermes_memory::{Experience, MemoryStore, Reflection};
use tracing::{debug, info, warn};

/// 反思系统
pub struct ReflectionSystem {
    memory: Arc<MemoryStore>,
}

impl ReflectionSystem {
    /// 创建反思系统
    pub fn new(memory: Arc<MemoryStore>) -> Self {
        Self { memory }
    }

    /// 执行反思
    pub async fn reflect(&self) -> Result<Reflection, hermes_core::HermesError> {
        let now_time = now();
        
        // 获取最近的经验（过去24小时）
        let recent_experiences = self.memory.recent_experiences(1000).await?;
        
        // 分析成功率（partial 成功率预留用于未来细粒度评估）
        let (successes, failures, _partial) = self.analyze_outcomes(&recent_experiences);
        
        // 生成洞察
        let insights = self.generate_insights(&recent_experiences).await?;
        
        // 生成改进建议
        let suggestions = self.generate_suggestions(&recent_experiences, &insights).await?;
        
        let period_start = recent_experiences.last()
            .map(|e| e.timestamp)
            .unwrap_or(now_time);

        let reflection = Reflection {
            timestamp: now_time,
            period_start,
            period_end: now_time,
            experiences_reviewed: recent_experiences.len(),
            successes,
            failures,
            insights,
            suggested_improvements: suggestions,
        };

        info!(
            "反思完成: {} 经验, {} 成功, {} 失败",
            reflection.experiences_reviewed,
            reflection.successes,
            reflection.failures
        );

        Ok(reflection)
    }

    /// 分析结果
    fn analyze_outcomes(&self, experiences: &[Experience]) -> (usize, usize, usize) {
        let mut successes = 0;
        let mut failures = 0;
        let mut partial = 0;

        for exp in experiences {
            match exp.evaluation {
                Evaluation::Success => successes += 1,
                Evaluation::Failure => failures += 1,
                Evaluation::PartialSuccess => partial += 1,
            }
        }

        (successes, failures, partial)
    }

    /// 生成洞察
    async fn generate_insights(
        &self,
        experiences: &[Experience],
    ) -> Result<Vec<String>, hermes_core::HermesError> {
        let mut insights = vec![];

        // 1. 整体成功率分析
        let total = experiences.len();
        if total > 0 {
            let success_count = experiences.iter()
                .filter(|e| e.evaluation == Evaluation::Success)
                .count();
            let rate = success_count as f64 / total as f64;

            if rate > 0.9 {
                insights.push(format!("成功率优秀: {:.1}%", rate * 100.0));
            } else if rate < 0.5 {
                insights.push(format!("成功率较低: {:.1}%，需要改进", rate * 100.0));
            } else {
                insights.push(format!("成功率正常: {:.1}%", rate * 100.0));
            }
        }

        // 2. 失败模式分析
        let failures: Vec<_> = experiences.iter()
            .filter(|e| e.evaluation == Evaluation::Failure)
            .collect();

        if !failures.is_empty() {
            insights.push(format!("检测到 {} 次失败", failures.len()));
            
            // 分析失败类型
            let permission_errors = failures.iter()
                .filter(|e| e.outcome.message.contains("权限") || e.outcome.message.contains("permission"))
                .count();
            
            if permission_errors > 0 {
                insights.push(format!("其中 {} 次是权限问题", permission_errors));
            }
        }

        // 3. 技能使用情况
        let skills_used = experiences.iter()
            .filter_map(|e| e.extracted_skill)
            .count();
        
        if skills_used > 0 {
            insights.push(format!("使用了 {} 次技能", skills_used));
        } else if total > 10 {
            insights.push("尚未从经验中提取技能，建议启用学习模式".to_string());
        }

        Ok(insights)
    }

    /// 生成改进建议
    #[allow(unused_variables)] // insights 预留用于基于洞察的个性化建议
    async fn generate_suggestions(
        &self,
        experiences: &[Experience],
        _insights: &[String],
    ) -> Result<Vec<String>, hermes_core::HermesError> {
        let mut suggestions = vec![];

        let total = experiences.len();
        let success_count = experiences.iter()
            .filter(|e| e.evaluation == Evaluation::Success)
            .count();

        // 根据成功率提出建议
        if total > 0 {
            let rate = success_count as f64 / total as f64;
            
            if rate < 0.5 {
                suggestions.push("成功率较低，建议：".to_string());
                suggestions.push("  1. 检查权限配置".to_string());
                suggestions.push("  2. 增加输入验证".to_string());
                suggestions.push("  3. 添加更多错误处理".to_string());
            }
        }

        // 检查是否有频繁失败的模式
        let recent_failures: Vec<_> = experiences.iter()
            .filter(|e| e.evaluation == Evaluation::Failure)
            .take(5)
            .collect();

        if recent_failures.len() >= 3 {
            suggestions.push("最近多次失败，建议暂停并检查问题".to_string());
        }

        // 建议提取技能
        let unprocessed_experiences = experiences.iter()
            .filter(|e| e.extracted_skill.is_none())
            .count();

        if unprocessed_experiences > 5 {
            suggestions.push(format!(
                "有 {} 条经验未提取技能，建议进行技能学习",
                unprocessed_experiences
            ));
        }

        Ok(suggestions)
    }

    /// 深度反思
    pub async fn deep_reflect(&self) -> Result<DeepReflection, hermes_core::HermesError> {
        let model = self.memory.load_self_model().await?;
        let skills = self.memory.list_skills().await?;
        let recent_reflections = self.memory.recent_reflections(5).await?;

        // 计算趋势
        let trend = if recent_reflections.len() >= 2 {
            let recent = &recent_reflections[0];
            let older = &recent_reflections[recent_reflections.len() - 1];
            
            if recent.successes > older.successes {
                Trend::Improving
            } else if recent.successes < older.successes {
                Trend::Declining
            } else {
                Trend::Stable
            }
        } else {
            Trend::Unknown
        };

        Ok(DeepReflection {
            identity: model.identity,
            total_skills: skills.len(),
            average_skill_proficiency: if skills.is_empty() {
                0.0
            } else {
                skills.iter().map(|s| s.proficiency).sum::<f32>() / skills.len() as f32
            },
            trend,
            last_reflection: recent_reflections.first().map(|r| r.timestamp),
        })
    }
}

/// 深度反思结果
#[derive(Debug, Clone)]
pub struct DeepReflection {
    pub identity: hermes_memory::Identity,
    pub total_skills: usize,
    pub average_skill_proficiency: f32,
    pub trend: Trend,
    pub last_reflection: Option<Timestamp>,
}

/// 趋势
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Improving,  // 改进中
    Stable,     // 稳定
    Declining,  // 下降
    Unknown,    // 未知
}

impl std::fmt::Display for DeepReflection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== 深度反思 ===")?;
        writeln!(f, "身份: {}", self.identity.name)?;
        writeln!(f, "版本: {}", self.identity.version)?;
        writeln!(f, "技能数: {}", self.total_skills)?;
        writeln!(f, "平均熟练度: {:.1}%", self.average_skill_proficiency * 100.0)?;
        writeln!(f, "趋势: {:?}", self.trend)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_core::Context;

    // 测试需要 MemoryStore，可能需要集成测试
}
