use crate::error::Result;
use crate::expr::Variable;
use crate::hir::{Block, Program};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct LiveVariables {
    live_in: HashMap<usize, HashSet<Variable>>,
    live_out: HashMap<usize, HashSet<Variable>>,
}

impl LiveVariables {
    /// Returns the set of variables which are live on basic block entry.
    pub fn live_at_entry(&self, block_index: usize) -> Result<&HashSet<Variable>> {
        self.live_in
            .get(&block_index)
            .ok_or_else(|| format!("Basic block with index {} does not exist", block_index).into())
    }

    /// Returns the set of variables which are live on basic block exit.
    pub fn live_at_exit(&self, block_index: usize) -> Result<&HashSet<Variable>> {
        self.live_out
            .get(&block_index)
            .ok_or_else(|| format!("Basic block with index {} does not exist", block_index).into())
    }
}

/// Computes the set of live variables for each basic block.
///
/// This analysis is limited to DAGs only.
pub fn live_variables(program: &Program) -> Result<LiveVariables> {
    let cfg = program.control_flow_graph();

    // Tracks the live variables for each block.
    let mut live_in: HashMap<usize, HashSet<Variable>> = HashMap::new();
    let mut live_out: HashMap<usize, HashSet<Variable>> = HashMap::new();

    // As the analysis is limited to DAGs only, the backwards may analysis can be done in reversed
    // top-sort ordering without work list.
    let top_sort = cfg.graph().compute_topological_ordering()?;
    for &block_index in top_sort.iter().rev() {
        let block = cfg.block(block_index)?;
        let (gen, kill) = compute_use_def_set(block);

        // out = union of all successor live variables
        let mut out: HashSet<Variable> = HashSet::new();
        for successor in cfg.successor_indices(block_index)? {
            live_in.get(&successor).unwrap().iter().for_each(|var| {
                out.insert(var.to_owned());
            });
        }

        // inp = gen U (out / kill)
        let mut inp: HashSet<Variable> = out.clone();
        kill.iter().for_each(|var| {
            inp.remove(var);
        });
        gen.into_iter().for_each(|var| {
            inp.insert(var.to_owned());
        });

        live_in.insert(block_index, inp);
        live_out.insert(block_index, out);
    }

    Ok(LiveVariables { live_in, live_out })
}

fn compute_use_def_set(block: &Block) -> (HashSet<&Variable>, HashSet<&Variable>) {
    let mut used = HashSet::new();
    let mut defined = HashSet::new();

    for phi_node in block.phi_nodes() {
        phi_node.incoming_variables().into_iter().for_each(|var| {
            used.insert(var);
        });

        defined.insert(phi_node.out());
    }

    for inst in block.instructions() {
        inst.variables_read()
            .into_iter()
            .filter(|var| !defined.contains(var))
            .for_each(|var| {
                used.insert(var);
            });

        inst.variables_written().iter().for_each(|var| {
            defined.insert(var);
        });
    }

    (used, defined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::Boolean;
    use crate::hir::{ControlFlowGraph, PhiNode};

    #[test]
    fn test_compute_use_def_set() {
        // GIVEN
        let mut phi_node = PhiNode::new(Boolean::variable("a"));
        phi_node.add_incoming(Boolean::variable("b"), 0);

        let mut block = Block::new(1);
        // use: {} | def: {}
        block.add_phi_node(phi_node);
        // use: {b} | def: {a}
        block
            .assign(Boolean::variable("c"), Boolean::variable("a").into())
            .unwrap();
        // use: {b} | def: {a, c}
        block
            .assign(Boolean::variable("d"), Boolean::variable("e").into())
            .unwrap();
        // use: {b, e} | def: {a, c, d}
        block.assert(Boolean::variable("d").into()).unwrap();
        // use: {b, e} | def: {a, c, d}

        // WHEN
        let (used, defined) = compute_use_def_set(&block);

        // THEN
        assert_eq!(
            used,
            vec![&Boolean::variable("b"), &Boolean::variable("e")]
                .into_iter()
                .collect()
        );
        assert_eq!(
            defined,
            vec![
                &Boolean::variable("a"),
                &Boolean::variable("c"),
                &Boolean::variable("d")
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn test_live_variables() {
        // GIVEN
        //         block 0
        //   +---+ a = y +---+
        //   |               |
        //   v               v
        // block 1         block 2
        // x = a           x = b
        // y = x             +
        //   |               |
        //   +-------+-------+
        //           |
        //           v
        //        block 3
        //         y = c
        //         z = x
        let mut cfg = ControlFlowGraph::new();

        let block0 = cfg.new_block().unwrap();
        block0
            .assign(Boolean::variable("a"), Boolean::variable("y").into())
            .unwrap();

        let block1 = cfg.new_block().unwrap();
        block1
            .assign(Boolean::variable("x"), Boolean::variable("a").into())
            .unwrap();
        block1
            .assign(Boolean::variable("y"), Boolean::variable("x").into())
            .unwrap();

        let block2 = cfg.new_block().unwrap();
        block2
            .assign(Boolean::variable("x"), Boolean::variable("b").into())
            .unwrap();

        let block3 = cfg.new_block().unwrap();
        block3
            .assign(Boolean::variable("y"), Boolean::variable("c").into())
            .unwrap();

        block3
            .assign(Boolean::variable("z"), Boolean::variable("x").into())
            .unwrap();

        cfg.set_entry(0).unwrap();

        cfg.unconditional_edge(0, 1).unwrap();
        cfg.unconditional_edge(0, 2).unwrap();
        cfg.unconditional_edge(1, 3).unwrap();
        cfg.unconditional_edge(2, 3).unwrap();

        let program = Program::new(cfg);

        // WHEN
        let live_vars = live_variables(&program).unwrap();

        // WHEN
        assert_eq!(
            live_vars.live_at_entry(0).unwrap(),
            &vec![
                Boolean::variable("c"),
                Boolean::variable("y"),
                Boolean::variable("b")
            ]
            .into_iter()
            .collect::<HashSet<Variable>>()
        );
        assert_eq!(
            live_vars.live_at_exit(0).unwrap(),
            &vec![
                Boolean::variable("c"),
                Boolean::variable("a"),
                Boolean::variable("b")
            ]
            .into_iter()
            .collect::<HashSet<Variable>>()
        );

        assert_eq!(
            live_vars.live_at_entry(1).unwrap(),
            &vec![Boolean::variable("c"), Boolean::variable("a")]
                .into_iter()
                .collect::<HashSet<Variable>>()
        );
        assert_eq!(
            live_vars.live_at_exit(1).unwrap(),
            &vec![Boolean::variable("c"), Boolean::variable("x")]
                .into_iter()
                .collect::<HashSet<Variable>>()
        );

        assert_eq!(
            live_vars.live_at_entry(2).unwrap(),
            &vec![Boolean::variable("c"), Boolean::variable("b")]
                .into_iter()
                .collect::<HashSet<Variable>>()
        );
        assert_eq!(
            live_vars.live_at_exit(2).unwrap(),
            &vec![Boolean::variable("c"), Boolean::variable("x")]
                .into_iter()
                .collect::<HashSet<Variable>>()
        );

        assert_eq!(
            live_vars.live_at_entry(3).unwrap(),
            &vec![Boolean::variable("c"), Boolean::variable("x")]
                .into_iter()
                .collect::<HashSet<Variable>>()
        );
        assert_eq!(live_vars.live_at_exit(3).unwrap().is_empty(), true);
    }
}
