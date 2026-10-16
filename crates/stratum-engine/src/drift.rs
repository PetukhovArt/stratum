use camino::Utf8Path;
use stratum_config::Config;
use stratum_core::types::ContainerKind;

use crate::pipeline::EngineError;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Drift {
    pub layer_edges: Vec<LayerEdgeGrowth>,
    pub slice_edges: Vec<SliceEdge>,
    pub cycles: Vec<CycleChange>,
    pub stages: Vec<StageChange>,
    pub fan_in: Vec<FanInGrowth>,
}

impl Drift {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layer_edges.is_empty()
            && self.slice_edges.is_empty()
            && self.cycles.is_empty()
            && self.stages.is_empty()
            && self.fan_in.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerEdgeGrowth {
    pub from: String,
    pub to: String,
    pub base: usize,
    pub head: usize,
    pub declared: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceEdge {
    pub from: String,
    pub to: String,
    pub example: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleChange {
    pub members: Vec<String>,
    pub grown_from: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageChange {
    pub container: String,
    pub kind: ContainerKind,
    pub base: Option<u8>,
    pub head: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FanInGrowth {
    pub slice: String,
    pub base: usize,
    pub head: usize,
    pub added: Vec<String>,
}

#[expect(clippy::unnecessary_wraps, reason = "stub until the drift core slice")]
pub fn drift(
    _base_root: &Utf8Path,
    _head_root: &Utf8Path,
    _config: &Config,
) -> Result<Drift, EngineError> {
    Ok(Drift::default())
}
