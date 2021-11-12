#[derive(Debug, Clone, Default)]
pub struct APIFilteringStatsConfig {
    pub field_count: usize,
    pub const_count: usize,
    pub expression_count: usize,
    pub function_count: usize,
}
