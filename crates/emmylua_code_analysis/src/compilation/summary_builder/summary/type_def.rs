//! Workspace type index — 跨文件类型定义聚合。
//!
//! 设计要点：
//! - 类型可能在多个文件中定义（partial class、meta 文件、global 变量）
//! - 通过 `SummaryFileListInput` 自动失效重建（salsa memoized 每个文件）
//! - 查询 O(1) by name

use smol_str::SmolStr;

use super::SalsaDocTypeDefKindSummary;
use crate::compilation::SalsaDocVisibilityKindSummary;
use crate::FileId;

/// A single type definition entry in the workspace index.
#[derive(Debug, Clone, PartialEq, Eq, salsa::Update)]
pub struct TypeDefEntry {
    pub file_id: FileId,
    pub name: SmolStr,
    pub kind: SalsaDocTypeDefKindSummary,
    pub visibility: SalsaDocVisibilityKindSummary,
    pub syntax_offset: rowan::TextSize,
}

/// Workspace-aggregated type index: type_name → all definitions across files.
///
/// ## 跨文件类型规则
///
/// | 条件 | 行为 |
/// |------|------|
/// | class + `(partial)` | 允许多文件定义 |
/// | class + meta 文件 | 同 workspace 内覆盖 |
/// | enum | 禁止多文件定义 |
/// | alias | 禁止多文件定义 |
/// | global 变量 | 任意跨文件定义 |
///
/// ## 性能
///
/// 重建耗时 ∝ 总类型定义数（通常 < 1000）。每个文件的 `SalsaDocSummary.type_defs`
/// 已被 salsa memoized，类型未变的文件零开销（仅 Arc 引用）。
#[derive(Debug, Clone, PartialEq, Eq, salsa::Update)]
pub struct WorkspaceTypeIndex {
    pub by_name: Vec<(SmolStr, Vec<TypeDefEntry>)>,
}

impl WorkspaceTypeIndex {
    /// 根据类型名查找所有定义条目。
    pub fn find(&self, type_name: &str) -> Option<&[TypeDefEntry]> {
        self.by_name.iter()
            .find(|(n, _)| n.as_str() == type_name)
            .map(|(_, e)| e.as_slice())
    }

    /// 统计类型定义的唯一文件数。
    pub fn count_files(&self, type_name: &str) -> usize {
        self.find(type_name).map(|e| e.len()).unwrap_or(0)
    }
}
