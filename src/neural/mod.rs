//! Shared neural compilation: TIR graphs, numerical models, training and
//! candidate search. A warrior supplies instruction meaning through `Target`.

pub mod checkpoint;
pub mod data;
pub mod inference;
pub mod model;
pub mod target;
pub mod training;

#[cfg(test)]
mod tests;

use crate::tir::TIROp;
use burn::prelude::Backend;
use data::tir_graph::TirGraph;
use model::composite::{NeuralCompilerConfig, NeuralCompilerV2};
pub use target::{GrammarState, Target, Vocabulary};

pub struct CompileResult {
    pub assembly: Vec<String>,
    /// Measured in the target's cost unit.
    pub cost: u64,
    pub valid_count: usize,
    pub total_count: usize,
    pub neural: bool,
}

pub fn compile(
    target: &dyn Target,
    ops: &[TIROp],
    baseline: &[String],
) -> Result<CompileResult, String> {
    compile_with_device::<burn::backend::Wgpu>(target, ops, baseline, &Default::default())
}

pub fn compile_with_device<B: Backend>(
    target: &dyn Target,
    ops: &[TIROp],
    baseline: &[String],
    device: &B::Device,
) -> Result<CompileResult, String> {
    if checkpoint::available_checkpoints(&target.checkpoint_dir()).is_empty() {
        return Ok(fallback(target, baseline, 0, 0));
    }
    match load_model::<B>(target, device) {
        Some(model) => compile_with_model(target, ops, baseline, &model, device),
        None => Ok(fallback(target, baseline, 0, 0)),
    }
}

pub fn load_model<B: Backend>(
    target: &dyn Target,
    device: &B::Device,
) -> Option<NeuralCompilerV2<B>> {
    let config = NeuralCompilerConfig::new(target.vocabulary().size());
    let directory = target.checkpoint_dir();
    for tag in [
        checkpoint::CheckpointTag::Production,
        checkpoint::CheckpointTag::Stage1Best,
    ] {
        if let Ok(Some(model)) =
            checkpoint::load_checkpoint(&directory, config.init::<B>(device), tag, device)
        {
            return Some(model);
        }
    }
    None
}

pub fn compile_with_model<B: Backend>(
    target: &dyn Target,
    ops: &[TIROp],
    baseline: &[String],
    model: &NeuralCompilerV2<B>,
    device: &B::Device,
) -> Result<CompileResult, String> {
    let graph = TirGraph::from_tir_ops(ops);
    if graph.nodes.is_empty() {
        return Ok(fallback(target, baseline, 0, 0));
    }
    let features = training::supervised::graph_to_features::<B>(&graph, device);
    let (src, dst, types) = training::supervised::graph_to_edges::<B>(&graph, device);
    let beams = inference::beam::beam_search(
        target,
        &model.encoder,
        &model.decoder,
        features,
        src,
        dst,
        types,
        &inference::beam::BeamConfig::default(),
        0,
        device,
    );
    match inference::execute::validate_and_rank(&beams.sequences, target, baseline, 0) {
        Some(ranked) if ranked.cost < target.cost(baseline) => Ok(CompileResult {
            assembly: ranked.assembly,
            cost: ranked.cost,
            valid_count: ranked.valid_count,
            total_count: ranked.total_count,
            neural: true,
        }),
        Some(ranked) => Ok(fallback(
            target,
            baseline,
            ranked.valid_count,
            ranked.total_count,
        )),
        None => Ok(fallback(target, baseline, 0, beams.sequences.len())),
    }
}

fn fallback(
    target: &dyn Target,
    baseline: &[String],
    valid_count: usize,
    total_count: usize,
) -> CompileResult {
    CompileResult {
        assembly: baseline.to_vec(),
        cost: target.cost(baseline),
        valid_count,
        total_count,
        neural: false,
    }
}
