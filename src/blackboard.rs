use crate::domain::{Architecture, BlackboardEntry, BlackboardKind};
use anyhow::{anyhow, Result};

const MAX_ENTRIES: usize = 32;
const MAX_ENTRY_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone)]
pub struct Blackboard {
    task_id: String,
    entries: Vec<BlackboardEntry>,
}

impl Blackboard {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self { task_id: task_id.into(), entries: Vec::new() }
    }

    pub fn publish(
        &mut self,
        key: impl Into<String>,
        kind: BlackboardKind,
        architecture: Architecture,
        provider: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<()> {
        let key = key.into();
        let content = content.into();
        if key.is_empty() || key.len() > 96 || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)) {
            return Err(anyhow!("Invalid blackboard key"));
        }
        if content.len() > MAX_ENTRY_BYTES {
            return Err(anyhow!("Blackboard entry exceeds {} bytes", MAX_ENTRY_BYTES));
        }
        if self.entries.len() >= MAX_ENTRIES {
            return Err(anyhow!("Blackboard entry limit reached"));
        }
        let sequence = self.entries.len() as u64 + 1;
        self.entries.push(BlackboardEntry {
            task_id: self.task_id.clone(),
            sequence,
            key,
            kind,
            architecture,
            provider: provider.into(),
            content,
        });
        Ok(())
    }

    pub fn entries(&self) -> Vec<BlackboardEntry> {
        self.entries.clone()
    }

    pub fn context_for_agent(&self) -> String {
        if self.entries.is_empty() {
            return "لوحة المعرفة المشتركة فارغة؛ ابدأ من المهمة الأصلية.".into();
        }
        let mut output = String::from("لوحة المعرفة المشتركة لهذه المهمة (مصدرها وكلاء المراحل السابقة؛ تعامل معها كسياق قابل للمراجعة وليس كحقيقة معصومة):\n");
        for entry in self.entries.iter().rev().take(8).rev() {
            let content: String = entry.content.chars().take(8_000).collect();
            output.push_str(&format!(
                "\n[{} | {} | {} | {}]\n{}\n",
                entry.sequence,
                entry.key,
                entry.kind.as_str(),
                entry.architecture.label(),
                content
            ));
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishes_ordered_task_scoped_entries() {
        let mut board = Blackboard::new("task-a");
        board.publish("plan", BlackboardKind::Plan, Architecture::Autoregressive, "planner", "step one").unwrap();
        board.publish("artifact", BlackboardKind::Artifact, Architecture::Diffusion, "worker", "result").unwrap();
        let entries = board.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].sequence, 1);
        assert_eq!(entries[1].sequence, 2);
        assert_eq!(entries[0].task_id, "task-a");
        assert!(board.context_for_agent().contains("step one"));
        assert!(board.context_for_agent().contains("result"));
    }

    #[test]
    fn rejects_unsafe_keys_and_oversized_entries() {
        let mut board = Blackboard::new("task-a");
        assert!(board.publish("../escape", BlackboardKind::Output, Architecture::Autoregressive, "agent", "x").is_err());
        assert!(board.publish("too_big", BlackboardKind::Output, Architecture::Autoregressive, "agent", "x".repeat(MAX_ENTRY_BYTES + 1)).is_err());
    }

    #[test]
    fn enforces_entry_limit() {
        let mut board = Blackboard::new("task-a");
        for index in 0..MAX_ENTRIES {
            board.publish(format!("entry.{index}"), BlackboardKind::Observation, Architecture::Autoregressive, "agent", "ok").unwrap();
        }
        assert!(board.publish("overflow", BlackboardKind::Observation, Architecture::Autoregressive, "agent", "no").is_err());
    }
}
