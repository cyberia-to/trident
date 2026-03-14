//! Unit tests for the TIR graph.

#[cfg(test)]
mod tests {
    use crate::ir::tir::TIROp;
    use crate::neural::data::tir_graph::*;

    #[test]
    fn simple_arithmetic_graph() {
        // push 3; push 4; add -> 3 nodes
        let ops = vec![TIROp::Push(3), TIROp::Push(4), TIROp::Add];
        let graph = TirGraph::from_tir_ops(&ops);

        assert_eq!(graph.num_nodes(), 3);
        // ControlFlow: push3->push4, push4->add = 2
        assert_eq!(graph.count_edges(EdgeKind::ControlFlow), 2);
        // DataDep: push3->add, push4->add = 2
        assert_eq!(graph.count_edges(EdgeKind::DataDep), 2);
        assert_eq!(graph.count_edges(EdgeKind::MemOrder), 0);
    }

    #[test]
    fn memory_ops_get_mem_order_edges() {
        let ops = vec![
            TIROp::Push(100),
            TIROp::ReadMem(1),
            TIROp::Push(200),
            TIROp::WriteMem(1),
        ];
        let graph = TirGraph::from_tir_ops(&ops);

        assert_eq!(graph.num_nodes(), 4);
        assert!(graph.count_edges(EdgeKind::MemOrder) >= 1);
    }

    #[test]
    fn if_else_creates_branch_edges() {
        let ops = vec![
            TIROp::Push(1), // condition
            TIROp::IfElse {
                then_body: vec![TIROp::Push(10)],
                else_body: vec![TIROp::Push(20)],
            },
        ];
        let graph = TirGraph::from_tir_ops(&ops);

        // 4 nodes: Push(1), IfElse, Push(10), Push(20)
        assert_eq!(graph.num_nodes(), 4);
        // ControlFlow edges include branch edges to both bodies
        let cf = graph.count_edges(EdgeKind::ControlFlow);
        assert!(cf >= 3, "expected >=3 CF edges, got {}", cf);
    }

    #[test]
    fn loop_creates_back_edge() {
        let ops = vec![
            TIROp::Push(5),
            TIROp::Loop {
                label: "l".into(),
                body: vec![TIROp::Push(1), TIROp::Sub],
            },
        ];
        let graph = TirGraph::from_tir_ops(&ops);

        // 4 nodes: Push(5), Loop, Push(1), Sub
        assert_eq!(graph.num_nodes(), 4);
        // Should have a back edge from Sub -> Loop
        let has_back_edge = graph
            .edges
            .iter()
            .any(|(from, to, kind)| *kind == EdgeKind::ControlFlow && *from == 3 && *to == 1);
        assert!(has_back_edge, "missing loop back edge");
    }

    #[test]
    fn feature_vector_dimensions() {
        let node = TirNode {
            op: OpKind::Add,
            field_type: FieldType::BFE,
            immediate: None,
        };
        let fv = node.feature_vector();
        assert_eq!(fv.len(), NODE_FEATURE_DIM);
        assert_eq!(fv.len(), 59);
        // Add is index 15
        assert_eq!(fv[15], 1.0);
        // BFE is index 54
        assert_eq!(fv[54], 1.0);
        // No immediate
        assert_eq!(fv[57], 0.0);
    }

    #[test]
    fn feature_vector_with_immediate() {
        let node = TirNode {
            op: OpKind::Push,
            field_type: FieldType::BFE,
            immediate: Some(42),
        };
        let fv = node.feature_vector();
        assert_eq!(fv[11], 1.0); // Push is index 11
        assert_eq!(fv[57], 1.0); // has_immediate = 1
        assert!(fv[58] > 0.0); // normalized immediate > 0
    }

    #[test]
    fn empty_ops_produces_empty_graph() {
        let graph = TirGraph::from_tir_ops(&[]);
        assert_eq!(graph.num_nodes(), 0);
        assert_eq!(graph.num_edges(), 0);
    }

    #[test]
    fn all_54_op_kinds_are_numbered() {
        assert_eq!(OpKind::Call as u8, 0);
        assert_eq!(OpKind::ProofBlock as u8, 53);
        assert_eq!(NUM_OP_KINDS, 54);
    }

    #[test]
    fn dup_creates_data_dep_without_consuming() {
        // push 7; dup 0 -> dup reads from push without consuming
        let ops = vec![TIROp::Push(7), TIROp::Dup(0)];
        let graph = TirGraph::from_tir_ops(&ops);

        assert_eq!(graph.num_nodes(), 2);
        // DataDep: push->dup (read dependency)
        assert_eq!(graph.count_edges(EdgeKind::DataDep), 1);
    }
}
