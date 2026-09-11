//! Machine-owned semantics used by the shared neural compiler.

use std::path::PathBuf;

/// A target vocabulary. Token zero is the sequence boundary token.
pub trait Vocabulary: Send + Sync {
    fn size(&self) -> usize;
    fn encode(&self, instruction: &str) -> Option<u32>;
    fn decode(&self, token: u32) -> Option<&str>;

    /// Refuse an unrepresentable sequence instead of silently removing its
    /// instructions and teaching the model a different program.
    fn encode_sequence(&self, lines: &[String]) -> Vec<u32> {
        let Some(mut tokens) = lines
            .iter()
            .map(|s| self.encode(s))
            .collect::<Option<Vec<_>>>()
        else {
            return Vec::new();
        };
        tokens.push(0);
        tokens
    }

    fn decode_sequence(&self, tokens: &[u32]) -> Vec<String> {
        tokens
            .iter()
            .take_while(|&&t| t != 0)
            .map(|&t| self.decode(t).map(str::to_string))
            .collect::<Option<Vec<_>>>()
            .unwrap_or_default()
    }
}

/// Abstract state features supplied by the target. The shared model embeds
/// depth in 65 buckets and consumes a 24-element target-defined type vector.
pub trait GrammarState {
    fn step(&mut self, token: u32);
    fn valid_mask(&self) -> Vec<f32>;
    fn type_encoding(&self) -> Vec<f32>;
    fn depth_for_embedding(&self, max_depth: usize) -> u32;
}

/// The warrior owns instruction meaning, validity and prices. The core owns
/// graph encoding, numerical models, search, training and checkpoint I/O.
pub trait Target: Send + Sync {
    fn vocabulary(&self) -> &dyn Vocabulary;
    fn grammar(&self, initial_depth: i32) -> Box<dyn GrammarState>;
    fn verify_block(&self, baseline: &[String], candidate: &[String], seed: u64) -> bool;
    fn cost(&self, assembly: &[String]) -> u64;
    /// Target-separated storage prevents loading another ISA's token weights.
    fn checkpoint_dir(&self) -> PathBuf;

    /// Machine-specific candidate rewrites. The core verifies each returned
    /// candidate again before admitting it as a training pair.
    fn augment(
        &self,
        _assembly: &[String],
        _seed: u64,
        _walk_variants: usize,
        _max_swaps: usize,
    ) -> Vec<Vec<String>> {
        Vec::new()
    }
}
