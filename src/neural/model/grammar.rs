//! Target-provided abstract execution states for teacher forcing.

use crate::neural::Target;

pub struct SequenceState {
    pub masks: Vec<Vec<f32>>,
    pub depths: Vec<u32>,
    pub type_states: Vec<Vec<f32>>,
}

pub fn precompute_sequence_state(
    target: &dyn Target,
    tokens: &[u32],
    initial_depth: i32,
) -> SequenceState {
    let mut state = target.grammar(initial_depth);
    let mut result = SequenceState {
        masks: Vec::with_capacity(tokens.len()),
        depths: Vec::with_capacity(tokens.len()),
        type_states: Vec::with_capacity(tokens.len()),
    };
    for &token in tokens {
        result.masks.push(state.valid_mask());
        result.depths.push(state.depth_for_embedding(65));
        result.type_states.push(state.type_encoding());
        state.step(token);
    }
    result
}
