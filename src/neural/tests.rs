//! Two vocabularies exercise the shared harness without a warrior dependency.
use super::*;
use burn::backend::{Autodiff, NdArray};
use std::path::PathBuf;

struct ToyTarget {
    tokens: Vec<&'static str>,
    unit_cost: u64,
    directory: PathBuf,
}

impl Vocabulary for ToyTarget {
    fn size(&self) -> usize {
        self.tokens.len()
    }
    fn encode(&self, s: &str) -> Option<u32> {
        self.tokens.iter().position(|&t| t == s).map(|i| i as u32)
    }
    fn decode(&self, t: u32) -> Option<&str> {
        self.tokens.get(t as usize).copied()
    }
}

struct ToyState {
    depth: i32,
    vocabulary: usize,
}
impl GrammarState for ToyState {
    fn step(&mut self, t: u32) {
        match t {
            1 | 2 => self.depth += 1,
            3 => self.depth -= 1,
            _ => {}
        }
    }
    fn valid_mask(&self) -> Vec<f32> {
        let mut mask = vec![0.0; self.vocabulary];
        if self.depth < 2 {
            mask[3] = -1e9;
        }
        mask
    }
    fn type_encoding(&self) -> Vec<f32> {
        vec![0.0; 24]
    }
    fn depth_for_embedding(&self, max: usize) -> u32 {
        self.depth.clamp(0, max as i32 - 1) as u32
    }
}

impl ToyTarget {
    fn value(&self, lines: &[String]) -> Option<Vec<u64>> {
        let mut stack = Vec::new();
        for line in lines {
            match self.encode(line)? {
                1 => stack.push(1),
                2 => stack.push(2),
                3 => {
                    let b = stack.pop()?;
                    let a = stack.pop()?;
                    stack.push(a + b);
                }
                _ => return None,
            }
        }
        Some(stack)
    }
}

impl Target for ToyTarget {
    fn vocabulary(&self) -> &dyn Vocabulary {
        self
    }
    fn grammar(&self, depth: i32) -> Box<dyn GrammarState> {
        Box::new(ToyState {
            depth,
            vocabulary: self.size(),
        })
    }
    fn verify_block(&self, baseline: &[String], candidate: &[String], _: u64) -> bool {
        self.value(baseline).is_some() && self.value(baseline) == self.value(candidate)
    }
    fn cost(&self, assembly: &[String]) -> u64 {
        assembly.len() as u64 * self.unit_cost
    }
    fn checkpoint_dir(&self) -> PathBuf {
        self.directory.clone()
    }
}

fn targets(directory: &std::path::Path) -> [ToyTarget; 2] {
    [
        ToyTarget {
            tokens: vec!["", "one", "two", "plus"],
            unit_cost: 1,
            directory: directory.join("first"),
        },
        ToyTarget {
            tokens: vec!["", "a", "b", "merge", "reserved"],
            unit_cost: 7,
            directory: directory.join("second"),
        },
    ]
}

fn model_config(vocabulary: usize) -> NeuralCompilerConfig {
    NeuralCompilerConfig::new(vocabulary)
        .with_d_model(16)
        .with_d_edge(4)
        .with_gnn_layers(1)
        .with_decoder_layers(1)
        .with_n_heads(2)
        .with_d_ff(32)
        .with_max_seq(8)
        .with_dropout(0.0)
}

#[test]
fn targets_control_candidate_decoding_validation_and_cost() {
    let temp = tempfile::tempdir().unwrap();
    for target in targets(temp.path()) {
        let baseline = vec![
            target.tokens[1].into(),
            target.tokens[1].into(),
            target.tokens[3].into(),
        ];
        let candidates = vec![vec![2], vec![1], vec![999]];
        let ranked =
            inference::execute::validate_and_rank(&candidates, &target, &baseline, 42).unwrap();
        assert_eq!(ranked.assembly, vec![target.tokens[2].to_string()]);
        assert_eq!(ranked.cost, target.unit_cost);
        assert_eq!((ranked.valid_count, ranked.total_count), (1, 3));
        let fallback = compile_with_device::<NdArray>(
            &target,
            &[TIROp::Push(1)],
            &baseline,
            &Default::default(),
        )
        .unwrap();
        assert!(!fallback.neural);
        assert_eq!(fallback.cost, 3 * target.unit_cost);
        assert_eq!(fallback.assembly, baseline);
    }
}

#[test]
fn unrepresentable_training_output_is_rejected_as_a_whole() {
    let temp = tempfile::tempdir().unwrap();
    let target = &targets(temp.path())[0];
    let blocks = vec![(
        vec![TIROp::Push(1)],
        vec!["one".into(), "unknown".into()],
        "bad".into(),
        2,
    )];
    assert!(data::pairs::extract_pairs(&blocks, target).is_empty());
    assert!(target.decode_sequence(&[1, 999]).is_empty());
}

#[test]
fn model_search_and_supervised_training_use_target_vocabulary_dimensions() {
    type B = Autodiff<NdArray>;
    let temp = tempfile::tempdir().unwrap();
    for target in targets(temp.path()) {
        let device = Default::default();
        let graph = TirGraph::from_tir_ops(&[TIROp::Push(1)]);
        let config = model_config(target.size());
        let model = config.init::<B>(&device);
        let pairs = data::pairs::extract_pairs(
            &[(
                vec![TIROp::Push(1)],
                vec![target.tokens[1].into()],
                "unit".into(),
                target.unit_cost,
            )],
            &target,
        );
        let mut optimizer = training::supervised::create_optimizer::<B>(
            &training::supervised::SupervisedConfig::default(),
        );
        let (trained, result) = training::supervised::train_epoch(
            &target,
            model,
            &pairs,
            &mut optimizer,
            0.001,
            &device,
        );
        assert!(result.avg_loss.is_finite());
        let features = training::supervised::graph_to_features::<B>(&graph, &device);
        let (src, dst, kinds) = training::supervised::graph_to_edges::<B>(&graph, &device);
        let beams = inference::beam::beam_search(
            &target,
            &trained.encoder,
            &trained.decoder,
            features,
            src,
            dst,
            kinds,
            &inference::beam::BeamConfig {
                k: 2,
                max_steps: 4,
                ..Default::default()
            },
            0,
            &device,
        );
        assert_eq!(beams.sequences.len(), 2);
        assert!(beams
            .sequences
            .iter()
            .flatten()
            .all(|&t| (t as usize) < target.size()));
        let state = model::grammar::precompute_sequence_state(&target, &[1, 1, 3], 0);
        assert_eq!(state.depths, [0, 1, 2]);
        assert_eq!(state.masks[0].len(), target.size());
    }
}

#[test]
fn gflownet_loss_backpropagates_into_model_parameters() {
    use burn::optim::GradientsParams;
    use burn::tensor::Tensor;
    type B = Autodiff<NdArray>;
    let temp = tempfile::tempdir().unwrap();
    let target = &targets(temp.path())[0];
    let device = Default::default();
    let model = model_config(target.size()).init::<B>(&device);
    let graph = TirGraph::from_tir_ops(&[TIROp::Push(1)]);
    let config = training::gflownet::GFlowNetConfig {
        max_seq_len: 4,
        ..Default::default()
    };
    // A constant log_z makes this test require a gradient path through the
    // policy itself. The previous CPU f32 log_pf produced no model gradients.
    let (loss, reward, _) = training::gflownet::gflownet_step(
        &model,
        &graph,
        &["one".into()],
        1,
        Tensor::<B, 1>::zeros([1], &device),
        0,
        &config,
        target,
        &device,
    );
    assert!(reward.is_finite());
    let gradients = GradientsParams::from_grads(loss.backward(), &model);
    assert!(
        !gradients.is_empty(),
        "policy model must receive training gradients"
    );
}
