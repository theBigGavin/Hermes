//! 技能系统 - 我的可复用能力

use hermes_core::{HermesError, Id, Intent, Result, Timestamp, now};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::HashMap;

/// 技能ID
pub type SkillId = Id;

/// 技能 - 可复用的能力单元
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: SkillId,
    pub name: String,
    pub description: String,
    pub created_at: Timestamp,
    pub last_used: Timestamp,
    pub usage_count: u64,
    pub success_count: u64,
    pub proficiency: f32, // 0.0 - 1.0
    pub implementation: SkillImplementation,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillImplementation {
    /// 简单函数（描述如何实现）
    Description(String),
    /// 代码模板
    CodeTemplate { language: String, template: String },
    /// 命令序列
    CommandSequence(Vec<String>),
    /// 组合其他技能
    Composite(Vec<SkillId>),
}

impl Skill {
    pub fn new(name: impl Into<String>, implementation: SkillImplementation) -> Self {
        let now = now();
        Self {
            id: SkillId::new(),
            name: name.into(),
            description: String::new(),
            created_at: now,
            last_used: now,
            usage_count: 0,
            success_count: 0,
            proficiency: 0.5, // 初始熟练度
            implementation,
            tags: vec![],
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 记录使用
    pub fn record_usage(&mut self, success: bool) {
        self.usage_count += 1;
        self.last_used = now();
        
        if success {
            self.success_count += 1;
        }

        // 更新熟练度
        let success_rate = self.success_count as f32 / self.usage_count as f32;
        let experience_factor = (self.usage_count as f32 / 100.0).min(1.0);
        self.proficiency = success_rate * 0.7 + experience_factor * 0.3;
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f32 {
        if self.usage_count == 0 {
            0.0
        } else {
            self.success_count as f32 / self.usage_count as f32
        }
    }
}

/// 技能注册表
pub struct SkillRegistry {
    db: Db,
}

impl SkillRegistry {
    pub fn new(db: &Db) -> Self {
        Self { db: db.clone() }
    }

    /// 注册技能
    pub async fn register(&self, skill: &Skill) -> Result<()> {
        let key = format!("skill:{}", skill.id);
        let value = serde_json::to_vec(skill)?;
        
        self.db.insert(key.as_bytes(), value)?;
        
        // 更新名称索引
        let name_key = format!("skill_name:{}", skill.name);
        self.db.insert(name_key.as_bytes(), skill.id.to_string().as_bytes())?;
        
        // 更新标签索引
        for tag in &skill.tags {
            let tag_key = format!("skill_tag:{}:{}", tag, skill.id);
            self.db.insert(tag_key.as_bytes(), b"")?;
        }

        self.db.flush_async().await?;
        
        tracing::info!("技能已注册: {} ({})", skill.name, skill.id);
        Ok(())
    }

    /// 获取技能
    pub async fn get(&self, id: SkillId) -> Result<Option<Skill>> {
        let key = format!("skill:{}", id);
        
        match self.db.get(key.as_bytes())? {
            Some(data) => {
                let skill: Skill = serde_json::from_slice(&data)?;
                Ok(Some(skill))
            }
            None => Ok(None),
        }
    }

    /// 通过名称查找技能
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Skill>> {
        let name_key = format!("skill_name:{}", name);
        
        match self.db.get(name_key.as_bytes())? {
            Some(data) => {
                let id_str = String::from_utf8_lossy(&data);
                let id = id_str.parse().map_err(|_| HermesError::Other("无效的技能ID".to_string()))?;
                self.get(id).await
            }
            None => Ok(None),
        }
    }

    /// 通过意图查找技能
    pub async fn find_by_intent(&self, intent: &Intent) -> Result<Vec<Skill>> {
        let mut skills = vec![];
        
        // 简化的实现：通过标签匹配
        let intent_str = format!("{:?}", intent).to_lowercase();
        
        for item in self.db.scan_prefix(b"skill:") {
            let (_, value) = item?;
            let skill: Skill = serde_json::from_slice(&value)?;
            
            // 检查名称或描述是否匹配
            if skill.name.to_lowercase().contains(&intent_str)
                || skill.description.to_lowercase().contains(&intent_str)
                || skill.tags.iter().any(|t| intent_str.contains(&t.to_lowercase()))
            {
                skills.push(skill);
            }
        }

        // 按熟练度排序
        skills.sort_by(|a, b| b.proficiency.partial_cmp(&a.proficiency).unwrap());
        
        Ok(skills)
    }

    /// 通过标签查找技能
    pub async fn find_by_tag(&self, tag: &str) -> Result<Vec<Skill>> {
        let mut skills = vec![];
        let prefix = format!("skill_tag:{}:", tag);
        
        for item in self.db.scan_prefix(prefix.as_bytes()) {
            let (key, _) = item?;
            let key_str = String::from_utf8_lossy(&key);
            
            // 提取技能ID
            if let Some(id_str) = key_str.strip_prefix(&prefix) {
                if let Ok(id) = id_str.parse() {
                    if let Some(skill) = self.get(id).await? {
                        skills.push(skill);
                    }
                }
            }
        }

        Ok(skills)
    }

    /// 列出所有技能
    pub async fn list_all(&self) -> Result<Vec<Skill>> {
        let mut skills = vec![];
        
        for item in self.db.scan_prefix(b"skill:") {
            let (key, value) = item?;
            
            // 确保是技能记录（不是索引）
            let key_str = String::from_utf8_lossy(&key);
            if key_str.starts_with("skill:") && !key_str.contains("_name") && !key_str.contains("_tag") {
                let skill: Skill = serde_json::from_slice(&value)?;
                skills.push(skill);
            }
        }

        Ok(skills)
    }

    /// 统计技能数量
    pub async fn count(&self) -> Result<usize> {
        let count = self.db.scan_prefix(b"skill:")
            .filter(|item| {
                if let Ok((key, _)) = item {
                    let key_str = String::from_utf8_lossy(&key);
                    key_str.starts_with("skill:") && !key_str.contains("_name") && !key_str.contains("_tag")
                } else {
                    false
                }
            })
            .count();
        
        Ok(count)
    }

    /// 更新技能（记录使用）
    pub async fn record_usage(&self, id: SkillId, success: bool) -> Result<()> {
        if let Some(mut skill) = self.get(id).await? {
            skill.record_usage(success);
            self.register(&skill).await?;
        }
        Ok(())
    }
}

/// 技能图 - 技能之间的关系
pub struct SkillGraph {
    /// 依赖关系（已实现）
    dependencies: HashMap<SkillId, Vec<SkillId>>,
    /// 组合关系（预留 - 用于技能组合功能）
    #[allow(dead_code)]
    compositions: HashMap<SkillId, Vec<SkillId>>,
}

impl SkillGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            compositions: HashMap::new(),
        }
    }

    /// 添加依赖关系（skill 依赖 dependency）
    pub fn add_dependency(&mut self, skill: SkillId, dependency: SkillId) {
        self.dependencies
            .entry(skill)
            .or_default()
            .push(dependency);
    }

    /// 获取技能的所有依赖
    pub fn get_dependencies(&self, skill: SkillId) -> Option<&Vec<SkillId>> {
        self.dependencies.get(&skill)
    }

    /// 拓扑排序（用于确定执行顺序）
    pub fn topological_sort(&self, skills: &[SkillId]) -> Option<Vec<SkillId>> {
        // 简化的拓扑排序实现
        let mut result = vec![];
        let mut visited = std::collections::HashSet::new();
        let mut temp_mark = std::collections::HashSet::new();

        for skill in skills {
            if !self.visit(*skill, &mut visited, &mut temp_mark, &mut result) {
                return None; // 存在循环依赖
            }
        }

        Some(result)
    }

    fn visit(
        &self,
        skill: SkillId,
        visited: &mut std::collections::HashSet<SkillId>,
        temp_mark: &mut std::collections::HashSet<SkillId>,
        result: &mut Vec<SkillId>,
    ) -> bool {
        if temp_mark.contains(&skill) {
            return false; // 循环依赖
        }

        if visited.contains(&skill) {
            return true;
        }

        temp_mark.insert(skill);

        if let Some(deps) = self.dependencies.get(&skill) {
            for dep in deps {
                if !self.visit(*dep, visited, temp_mark, result) {
                    return false;
                }
            }
        }

        temp_mark.remove(&skill);
        visited.insert(skill);
        result.push(skill);

        true
    }
}

impl Default for SkillGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_creation() {
        let skill = Skill::new(
            "test_skill",
            SkillImplementation::Description("测试技能".to_string()),
        );

        assert_eq!(skill.name, "test_skill");
        assert_eq!(skill.proficiency, 0.5);
        assert_eq!(skill.usage_count, 0);
    }

    #[test]
    fn test_skill_usage() {
        let mut skill = Skill::new(
            "test",
            SkillImplementation::Description("test".to_string()),
        );

        skill.record_usage(true);
        skill.record_usage(true);
        skill.record_usage(false);

        assert_eq!(skill.usage_count, 3);
        assert_eq!(skill.success_count, 2);
        assert!(skill.proficiency > 0.0);
    }

    #[test]
    fn test_skill_graph() {
        let mut graph = SkillGraph::new();
        
        let a = SkillId::new();
        let b = SkillId::new();
        let c = SkillId::new();

        graph.add_dependency(a, b); // a 依赖 b
        graph.add_dependency(b, c); // b 依赖 c

        let sorted = graph.topological_sort(&[a, b, c]);
        assert!(sorted.is_some());
        
        let sorted = sorted.unwrap();
        assert_eq!(sorted[0], c); // c 最先执行
        assert_eq!(sorted[1], b);
        assert_eq!(sorted[2], a); // a 最后执行
    }
}
