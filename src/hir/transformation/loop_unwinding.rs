use crate::environment::UnwindingGuard;
use crate::error::Result;
use crate::hir::{ControlFlowGraph, RemovedEdgeGuard};
use crate::ir::Transform;
use falcon::graph::LoopTree;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Builder, Debug)]
pub struct LoopUnwinding {
    /// unwinding bound for a specific loop
    unwinding_bound: BTreeMap<usize, usize>,
    default_unwinding_bound: usize,
    unwinding_guard: UnwindingGuard,
}

impl LoopUnwinding {
    fn unwinding_bound_for_loop(&self, loop_id: usize) -> usize {
        self.unwinding_bound
            .get(&loop_id)
            .copied()
            .unwrap_or(self.default_unwinding_bound)
    }

    fn unwind_loop(
        &self,
        cfg: &mut ControlFlowGraph,
        loop_header: usize,
        loop_nodes: &BTreeSet<usize>,
    ) -> Result<BTreeSet<usize>> {
        // Compute all loops nodes which have an outgoing edge (aka back edge) to the loop header
        let back_nodes: Vec<usize> = cfg
            .predecessor_indices(loop_header)?
            .into_iter()
            .filter(|node| loop_nodes.contains(node))
            .collect();

        let loop_id = cfg.block(loop_header).unwrap().loop_id().unwrap();
        let k = self.unwinding_bound_for_loop(loop_id);

        if k == 0 {
            // No unwinding, only delete back edges to get rid of the loop and we are done
            for &back_node in &back_nodes {
                cfg.remove_edge(back_node, loop_header, self.removed_edge_guard())?;
            }
            return Ok(loop_nodes.clone());
        }

        // Loop unwinding adds additional nodes, collect them
        let mut loop_nodes_unwound = loop_nodes.clone();

        // First, create a copy for the last iteration.
        // All back edges of the last iteration are removed (replaced by unwinding assumptions).
        let last_loop_header = {
            let new_block_indices = cfg.duplicate_blocks(loop_nodes)?;
            let last_loop_header = new_block_indices[&loop_header];

            // Remove back edges
            for back_node in &back_nodes {
                let back_node = new_block_indices[back_node];
                cfg.remove_edge(back_node, last_loop_header, self.removed_edge_guard())?;
            }

            // Collect the newly created nodes
            for &new_block_id in new_block_indices.values() {
                loop_nodes_unwound.insert(new_block_id);
            }

            last_loop_header
        };

        // Then repeatedly duplicate the loop nodes for the remaining k-2 iterations.
        // The back edges of iteration i are rewired to the iteration i+1.
        let mut next_loop_header = last_loop_header;
        for _ in 1..k {
            let new_block_indices = cfg.duplicate_blocks(loop_nodes)?;
            let current_loop_header = new_block_indices[&loop_header];

            // Rewire the back edges of the current iteration to the loop header of the next iteration.
            for back_node in &back_nodes {
                let back_node = new_block_indices[back_node];
                cfg.rewire_edge(back_node, current_loop_header, back_node, next_loop_header)?;
            }

            // Collect the newly created nodes
            for &new_block_id in new_block_indices.values() {
                loop_nodes_unwound.insert(new_block_id);
            }

            next_loop_header = current_loop_header;
        }

        // Finally, rewire the first iteration to the second iteration to get rid of the loop.
        for back_node in &back_nodes {
            cfg.rewire_edge(*back_node, loop_header, *back_node, next_loop_header)?;
        }

        Ok(loop_nodes_unwound)
    }

    pub fn loop_tree(cfg: &ControlFlowGraph) -> Result<LoopTree> {
        let entry = cfg.entry()?;

        if !cfg.graph().is_reducible(entry)? {
            println!("Warning: CFG is not reducible!");
        }

        Ok(cfg.graph().compute_loop_tree(entry)?)
    }

    pub fn unwind_cfg(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let loop_tree = Self::loop_tree(cfg)?;
        let parent_loop_ids = loop_tree.compute_predecessors()?;
        let loops = loop_tree.vertices();

        // First set the loop IDs of all loop header blocks to keep track of them during unwinding.
        // Block duplication causes that head block IDs and loop IDs will diverge.
        for l in &loops {
            let header_block = cfg.block_mut(l.header())?;
            header_block.set_loop_id(Some(l.header()));
        }

        // Initialize the set of nodes for each loop.
        // The set of nodes for each loop will grow during unwinding.
        let mut all_loop_nodes: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for l in loops {
            all_loop_nodes.insert(l.header(), l.nodes().clone());
        }

        // Unwind the loops in reverse topsort ordering, i.e. starting from the innermost loop
        let top_sort = loop_tree.compute_topological_ordering()?;
        for &loop_header in top_sort.iter().rev() {
            let loop_nodes = &all_loop_nodes[&loop_header];

            let loop_nodes_unwound = self.unwind_loop(cfg, loop_header, loop_nodes)?;
            assert!(loop_nodes_unwound.is_superset(loop_nodes));

            // Now push all newly created loop nodes to the parent loops
            for parent_loop_id in &parent_loop_ids[&loop_header] {
                all_loop_nodes
                    .get_mut(parent_loop_id)
                    .unwrap()
                    .extend(&loop_nodes_unwound);
            }

            all_loop_nodes.insert(loop_header, loop_nodes_unwound);
        }

        // Loop unwinding may leave behind blocks which are dead ends,
        // meaning that no path from the block to the CFG exit exists.
        // Remove all of them and add unwinding assumptions/assertions instead.
        cfg.remove_dead_end_blocks(self.removed_edge_guard())?;

        Ok(())
    }

    fn removed_edge_guard(&self) -> RemovedEdgeGuard {
        match self.unwinding_guard {
            UnwindingGuard::Assumption => RemovedEdgeGuard::AssumeEdgeNotTaken,
            UnwindingGuard::Assertion => RemovedEdgeGuard::AssertEdgeNotTaken,
        }
    }
}

impl Transform<ControlFlowGraph> for LoopUnwinding {
    fn name(&self) -> &'static str {
        "LoopUnwinding"
    }

    fn description(&self) -> String {
        format!("Unwind loops (k={})", self.default_unwinding_bound)
    }

    fn transform(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        self.unwind_cfg(cfg)?;
        cfg.simplify()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::expr::{Boolean, Expression, Sort, Variable};
    use crate::util::RenderGraph;

    use std::path::Path;

    fn add_block_with_id(cfg: &mut ControlFlowGraph, id: &str) -> usize {
        let block = cfg.new_block();
        block
            .assign(Variable::new(id, Sort::boolean()), Boolean::constant(true))
            .unwrap()
            .labels_mut()
            .pseudo();
        block.index()
    }

    fn add_block_with_id_and_assumption(
        cfg: &mut ControlFlowGraph,
        id: &str,
        assumption: Expression,
    ) -> usize {
        let block = cfg.new_block();
        block
            .assign(Variable::new(id, Sort::boolean()), Boolean::constant(true))
            .unwrap()
            .labels_mut()
            .pseudo();
        block.assume(assumption).unwrap().labels_mut().pseudo();
        block.index()
    }

    fn debug_cfg(
        test_name: &str,
        given_cfg: &ControlFlowGraph,
        unwound_cfg: &ControlFlowGraph,
        expected_cfg: &ControlFlowGraph,
    ) {
        given_cfg
            .render_to_file(Path::new(&format!("{}_given.dot", test_name)))
            .unwrap();
        unwound_cfg
            .render_to_file(Path::new(&format!("{}_unwound.dot", test_name)))
            .unwrap();
        expected_cfg
            .render_to_file(Path::new(&format!("{}_expected.dot", test_name)))
            .unwrap();
    }

    #[test]
    fn test_unwind_self_loop_zero_times() {
        let l: Expression = Variable::new("L", Sort::boolean()).into();
        let not_l = Boolean::not(l.clone()).unwrap();

        // Given: Self loop at block 0
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = add_block_with_id(&mut cfg, "c0");
            let block1_index = add_block_with_id(&mut cfg, "c1");

            cfg.conditional_edge(block0_index, block0_index, l).unwrap(); // loop
            cfg.conditional_edge(block0_index, block1_index, not_l.clone())
                .unwrap();

            cfg.block_mut(block0_index)
                .unwrap()
                .set_loop_id(Some(block0_index));

            cfg.set_entry(block0_index).unwrap();
            cfg.set_exit(block1_index).unwrap();

            cfg
        };

        // When: Unwind with k=0
        let unwinder = LoopUnwindingBuilder::default()
            .default_unwinding_bound(0)
            .unwinding_bound(BTreeMap::default())
            .unwinding_guard(UnwindingGuard::Assumption)
            .build()
            .unwrap();

        let mut unwound_cfg = given_cfg;
        unwinder.unwind_cfg(&mut unwound_cfg).unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = add_block_with_id_and_assumption(&mut cfg, "c0", not_l.clone());
            let block1_index = add_block_with_id(&mut cfg, "c1");

            cfg.conditional_edge(block0_index, block1_index, not_l)
                .unwrap();

            cfg.block_mut(block0_index)
                .unwrap()
                .set_loop_id(Some(block0_index));

            cfg.set_entry(block0_index).unwrap();
            cfg.set_exit(block1_index).unwrap();

            cfg
        };

        /*debug_cfg(
            "loop_unwinding_test_unwind_self_loop_zero_times",
            &given_cfg,
            &unwound_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, unwound_cfg);
    }

    #[test]
    fn test_unwind_self_loop_three_times() {
        let l: Expression = Variable::new("L", Sort::boolean()).into();
        let not_l = Boolean::not(l.clone()).unwrap();

        // Given: Self loop at block 0
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = add_block_with_id(&mut cfg, "c0");
            let block1_index = add_block_with_id(&mut cfg, "c1");

            cfg.conditional_edge(block0_index, block0_index, l.clone())
                .unwrap(); // loop
            cfg.conditional_edge(block0_index, block1_index, not_l.clone())
                .unwrap();

            cfg.block_mut(block0_index)
                .unwrap()
                .set_loop_id(Some(block0_index));

            cfg.set_entry(block0_index).unwrap();
            cfg.set_exit(block1_index).unwrap();

            cfg
        };

        // When: Unwind with k=3
        let unwinder = LoopUnwindingBuilder::default()
            .default_unwinding_bound(3)
            .unwinding_bound(BTreeMap::default())
            .unwinding_guard(UnwindingGuard::Assumption)
            .build()
            .unwrap();

        let mut unwound_cfg = given_cfg;
        unwinder.unwind_cfg(&mut unwound_cfg).unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = add_block_with_id(&mut cfg, "c0");
            let block1_index = add_block_with_id(&mut cfg, "c1");

            // Duplicated blocks
            let block2_index = add_block_with_id_and_assumption(&mut cfg, "c0", not_l.clone());
            let block3_index = add_block_with_id(&mut cfg, "c0");
            let block4_index = add_block_with_id(&mut cfg, "c0");

            cfg.conditional_edge(block0_index, block4_index, l.clone())
                .unwrap();
            cfg.conditional_edge(block0_index, block1_index, not_l.clone())
                .unwrap();
            cfg.conditional_edge(block4_index, block3_index, l.clone())
                .unwrap();
            cfg.conditional_edge(block4_index, block1_index, not_l.clone())
                .unwrap();
            cfg.conditional_edge(block3_index, block2_index, l).unwrap();
            cfg.conditional_edge(block3_index, block1_index, not_l.clone())
                .unwrap();
            cfg.conditional_edge(block2_index, block1_index, not_l)
                .unwrap();

            cfg.block_mut(block0_index)
                .unwrap()
                .set_loop_id(Some(block0_index));
            cfg.block_mut(block2_index)
                .unwrap()
                .set_loop_id(Some(block0_index));
            cfg.block_mut(block3_index)
                .unwrap()
                .set_loop_id(Some(block0_index));
            cfg.block_mut(block4_index)
                .unwrap()
                .set_loop_id(Some(block0_index));

            cfg.set_entry(block0_index).unwrap();
            cfg.set_exit(block1_index).unwrap();

            cfg
        };

        /*debug_cfg(
            "loop_unwinding_test_unwind_self_loop_three_times",
            &given_cfg,
            &unwound_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, unwound_cfg);
    }

    #[test]
    fn test_unwind_nested_loop_one_time() {
        let a: Expression = Variable::new("a", Sort::boolean()).into();
        let not_a = Boolean::not(a.clone()).unwrap();

        let b: Expression = Variable::new("b", Sort::boolean()).into();
        let not_b = Boolean::not(b.clone()).unwrap();

        // Given: Self loop at block 1 and loop 0,1,2
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = add_block_with_id(&mut cfg, "c0");
            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id(&mut cfg, "c2");
            let block3_index = add_block_with_id(&mut cfg, "c3");

            cfg.unconditional_edge(block0_index, block1_index).unwrap();
            cfg.conditional_edge(block1_index, block1_index, b.clone())
                .unwrap(); // loop
            cfg.conditional_edge(block1_index, block2_index, not_b.clone())
                .unwrap();
            cfg.conditional_edge(block2_index, block0_index, a.clone())
                .unwrap(); // loop
            cfg.conditional_edge(block2_index, block3_index, not_a.clone())
                .unwrap();

            cfg.block_mut(block0_index)
                .unwrap()
                .set_loop_id(Some(block0_index));
            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block0_index).unwrap();
            cfg.set_exit(block3_index).unwrap();

            cfg
        };

        // When: Unwind with k=1
        let unwinder = LoopUnwindingBuilder::default()
            .default_unwinding_bound(1)
            .unwinding_bound(BTreeMap::default())
            .unwinding_guard(UnwindingGuard::Assumption)
            .build()
            .unwrap();

        let mut unwound_cfg = given_cfg;
        unwinder.unwind_cfg(&mut unwound_cfg).unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0_index = add_block_with_id(&mut cfg, "c0");
            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id(&mut cfg, "c2");
            let block3_index = add_block_with_id(&mut cfg, "c3");
            let block4_index = add_block_with_id_and_assumption(&mut cfg, "c1", not_b.clone());
            let block5_index = add_block_with_id(&mut cfg, "c0");
            let block6_index = add_block_with_id(&mut cfg, "c1");
            let block7_index = add_block_with_id_and_assumption(&mut cfg, "c2", not_a.clone());
            let block8_index = add_block_with_id_and_assumption(&mut cfg, "c1", not_b.clone());

            cfg.unconditional_edge(block0_index, block1_index).unwrap();
            cfg.conditional_edge(block1_index, block2_index, not_b.clone())
                .unwrap();
            cfg.conditional_edge(block1_index, block4_index, b.clone())
                .unwrap();
            cfg.conditional_edge(block2_index, block3_index, not_a.clone())
                .unwrap();
            cfg.conditional_edge(block2_index, block5_index, a).unwrap();
            cfg.conditional_edge(block4_index, block2_index, not_b.clone())
                .unwrap();
            cfg.unconditional_edge(block5_index, block6_index).unwrap();
            cfg.conditional_edge(block6_index, block7_index, not_b.clone())
                .unwrap();
            cfg.conditional_edge(block6_index, block8_index, b).unwrap();
            cfg.conditional_edge(block7_index, block3_index, not_a)
                .unwrap();
            cfg.conditional_edge(block8_index, block7_index, not_b)
                .unwrap();

            cfg.block_mut(block0_index)
                .unwrap()
                .set_loop_id(Some(block0_index));
            cfg.block_mut(block5_index)
                .unwrap()
                .set_loop_id(Some(block0_index));
            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));
            cfg.block_mut(block4_index)
                .unwrap()
                .set_loop_id(Some(block1_index));
            cfg.block_mut(block6_index)
                .unwrap()
                .set_loop_id(Some(block1_index));
            cfg.block_mut(block8_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block0_index).unwrap();
            cfg.set_exit(block3_index).unwrap();

            cfg
        };

        /*debug_cfg(
            "loop_unwinding_test_unwind_nested_loop_one_time",
            &given_cfg,
            &unwound_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, unwound_cfg);
    }

    #[test]
    fn test_unwind_loop_1_2_with_loop_entry_1_and_loop_exit_2_zero_times() {
        let l: Expression = Variable::new("L", Sort::boolean()).into();
        let not_l = Boolean::not(l.clone()).unwrap();

        // Given:
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id(&mut cfg, "c2");
            let block3_index = add_block_with_id(&mut cfg, "c3");

            cfg.unconditional_edge(block1_index, block2_index).unwrap();
            cfg.conditional_edge(block2_index, block1_index, l).unwrap(); // loop
            cfg.conditional_edge(block2_index, block3_index, not_l.clone())
                .unwrap();

            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block1_index).unwrap();
            cfg.set_exit(block3_index).unwrap();

            cfg
        };

        // When: Unwind with k=0
        let unwinder = LoopUnwindingBuilder::default()
            .default_unwinding_bound(0)
            .unwinding_bound(BTreeMap::default())
            .unwinding_guard(UnwindingGuard::Assumption)
            .build()
            .unwrap();

        let mut unwound_cfg = given_cfg;
        unwinder.unwind_cfg(&mut unwound_cfg).unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id_and_assumption(&mut cfg, "c2", not_l.clone());
            let block3_index = add_block_with_id(&mut cfg, "c3");

            cfg.unconditional_edge(block1_index, block2_index).unwrap();
            cfg.conditional_edge(block2_index, block3_index, not_l)
                .unwrap();

            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block1_index).unwrap();
            cfg.set_exit(block3_index).unwrap();

            cfg
        };

        /*debug_cfg(
            "test_unwind_loop_1_2_with_loop_entry_1_and_loop_exit_2_zero_times",
            &given_cfg,
            &unwound_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, unwound_cfg);
    }

    #[test]
    fn test_unwind_loop_1_2_with_loop_entry_1_and_loop_exit_2_one_time() {
        let l: Expression = Variable::new("L", Sort::boolean()).into();
        let not_l = Boolean::not(l.clone()).unwrap();

        // Given:
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id(&mut cfg, "c2");
            let block3_index = add_block_with_id(&mut cfg, "c3");

            cfg.unconditional_edge(block1_index, block2_index).unwrap();
            cfg.conditional_edge(block2_index, block1_index, l.clone())
                .unwrap(); // loop
            cfg.conditional_edge(block2_index, block3_index, not_l.clone())
                .unwrap();

            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block1_index).unwrap();
            cfg.set_exit(block3_index).unwrap();

            cfg
        };

        // When: Unwind with k=1
        let unwinder = LoopUnwindingBuilder::default()
            .default_unwinding_bound(1)
            .unwinding_bound(BTreeMap::default())
            .unwinding_guard(UnwindingGuard::Assumption)
            .build()
            .unwrap();

        let mut unwound_cfg = given_cfg;
        unwinder.unwind_cfg(&mut unwound_cfg).unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id(&mut cfg, "c2");
            let block3_index = add_block_with_id(&mut cfg, "c3");
            let block4_index = add_block_with_id(&mut cfg, "c1");
            let block5_index = add_block_with_id_and_assumption(&mut cfg, "c2", not_l.clone());

            cfg.unconditional_edge(block1_index, block2_index).unwrap();
            cfg.conditional_edge(block2_index, block3_index, not_l.clone())
                .unwrap();
            cfg.conditional_edge(block2_index, block4_index, l).unwrap();
            cfg.unconditional_edge(block4_index, block5_index).unwrap();
            cfg.conditional_edge(block5_index, block3_index, not_l)
                .unwrap();

            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));
            cfg.block_mut(block4_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block1_index).unwrap();
            cfg.set_exit(block3_index).unwrap();

            cfg
        };

        /*debug_cfg(
            "test_unwind_loop_1_2_with_loop_entry_1_and_loop_exit_2_one_time",
            &given_cfg,
            &unwound_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, unwound_cfg);
    }

    #[test]
    fn test_unwind_loop_1_2_with_loop_entry_1_and_loop_exit_1_zero_times() {
        let l: Expression = Variable::new("L", Sort::boolean()).into();
        let not_l = Boolean::not(l.clone()).unwrap();

        // Given:
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id(&mut cfg, "c2");
            let block3_index = add_block_with_id(&mut cfg, "c3");

            cfg.conditional_edge(block1_index, block2_index, l).unwrap();
            cfg.conditional_edge(block1_index, block3_index, not_l.clone())
                .unwrap();
            cfg.unconditional_edge(block2_index, block1_index).unwrap(); // loop

            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block1_index).unwrap();
            cfg.set_exit(block3_index).unwrap();

            cfg
        };

        // When: Unwind with k=0
        let unwinder = LoopUnwindingBuilder::default()
            .default_unwinding_bound(0)
            .unwinding_bound(BTreeMap::default())
            .unwinding_guard(UnwindingGuard::Assumption)
            .build()
            .unwrap();

        let mut unwound_cfg = given_cfg;
        unwinder.unwind_cfg(&mut unwound_cfg).unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block1_index = add_block_with_id_and_assumption(&mut cfg, "c1", not_l.clone());
            let block2_index = add_block_with_id(&mut cfg, "c2");
            let block3_index = add_block_with_id(&mut cfg, "c3");

            // block2 is dead end -> remove
            cfg.remove_block(block2_index, RemovedEdgeGuard::AssumeEdgeNotTaken)
                .unwrap();

            cfg.conditional_edge(block1_index, block3_index, not_l)
                .unwrap();

            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block1_index).unwrap();
            cfg.set_exit(block3_index).unwrap();

            cfg
        };

        /*debug_cfg(
            "test_unwind_loop_1_2_with_loop_entry_1_and_loop_exit_1_zero_times",
            &given_cfg,
            &unwound_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, unwound_cfg);
    }

    #[test]
    fn test_unwind_loop_1_2_with_loop_entry_1_and_loop_exit_1_one_time() {
        let l: Expression = Variable::new("L", Sort::boolean()).into();
        let not_l = Boolean::not(l.clone()).unwrap();

        // Given:
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id(&mut cfg, "c2");
            let block3_index = add_block_with_id(&mut cfg, "c3");

            cfg.conditional_edge(block1_index, block2_index, l.clone())
                .unwrap();
            cfg.conditional_edge(block1_index, block3_index, not_l.clone())
                .unwrap();
            cfg.unconditional_edge(block2_index, block1_index).unwrap(); // loop

            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block1_index).unwrap();
            cfg.set_exit(block3_index).unwrap();

            cfg
        };

        // When: Unwind with k=1
        let unwinder = LoopUnwindingBuilder::default()
            .default_unwinding_bound(1)
            .unwinding_bound(BTreeMap::default())
            .unwinding_guard(UnwindingGuard::Assumption)
            .build()
            .unwrap();

        let mut unwound_cfg = given_cfg;
        unwinder.unwind_cfg(&mut unwound_cfg).unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id(&mut cfg, "c2");
            let block3_index = add_block_with_id(&mut cfg, "c3");
            let block4_index = add_block_with_id_and_assumption(&mut cfg, "c1", not_l.clone());

            cfg.conditional_edge(block1_index, block2_index, l).unwrap();
            cfg.conditional_edge(block1_index, block3_index, not_l.clone())
                .unwrap();
            cfg.unconditional_edge(block2_index, block4_index).unwrap();
            cfg.conditional_edge(block4_index, block3_index, not_l)
                .unwrap();

            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));
            cfg.block_mut(block4_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block1_index).unwrap();
            cfg.set_exit(block3_index).unwrap();

            cfg
        };

        /*debug_cfg(
            "test_unwind_loop_1_2_with_loop_entry_1_and_loop_exit_1_one_time",
            &given_cfg,
            &unwound_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, unwound_cfg);
    }

    #[test]
    fn test_unwind_loop_1_2_3_with_loop_entry_1_and_loop_exit_2_zero_times() {
        let l: Expression = Variable::new("L", Sort::boolean()).into();
        let not_l = Boolean::not(l.clone()).unwrap();

        // Given:
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id(&mut cfg, "c2");
            let block3_index = add_block_with_id(&mut cfg, "c3");
            let block4_index = add_block_with_id(&mut cfg, "c4");

            cfg.unconditional_edge(block1_index, block2_index).unwrap();
            cfg.conditional_edge(block2_index, block3_index, l).unwrap();
            cfg.conditional_edge(block2_index, block4_index, not_l.clone())
                .unwrap();
            cfg.unconditional_edge(block3_index, block1_index).unwrap(); // loop

            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block1_index).unwrap();
            cfg.set_exit(block4_index).unwrap();

            cfg
        };

        // When: Unwind with k=0
        let unwinder = LoopUnwindingBuilder::default()
            .default_unwinding_bound(0)
            .unwinding_bound(BTreeMap::default())
            .unwinding_guard(UnwindingGuard::Assumption)
            .build()
            .unwrap();

        let mut unwound_cfg = given_cfg;
        unwinder.unwind_cfg(&mut unwound_cfg).unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id_and_assumption(&mut cfg, "c2", not_l.clone());
            let block3_index = add_block_with_id(&mut cfg, "c3");
            let block4_index = add_block_with_id(&mut cfg, "c4");

            // block3 is dead end -> remove
            cfg.remove_block(block3_index, RemovedEdgeGuard::AssumeEdgeNotTaken)
                .unwrap();

            cfg.unconditional_edge(block1_index, block2_index).unwrap();
            cfg.conditional_edge(block2_index, block4_index, not_l)
                .unwrap();

            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block1_index).unwrap();
            cfg.set_exit(block4_index).unwrap();

            cfg
        };

        /*debug_cfg(
            "test_unwind_loop_1_2_3_with_loop_entry_1_and_loop_exit_2_zero_times",
            &given_cfg,
            &unwound_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, unwound_cfg);
    }

    #[test]
    fn test_unwind_loop_1_2_3_with_loop_entry_1_and_loop_exit_2_one_time() {
        let l: Expression = Variable::new("L", Sort::boolean()).into();
        let not_l = Boolean::not(l.clone()).unwrap();

        // Given:
        let given_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id(&mut cfg, "c2");
            let block3_index = add_block_with_id(&mut cfg, "c3");
            let block4_index = add_block_with_id(&mut cfg, "c4");

            cfg.unconditional_edge(block1_index, block2_index).unwrap();
            cfg.conditional_edge(block2_index, block3_index, l.clone())
                .unwrap();
            cfg.conditional_edge(block2_index, block4_index, not_l.clone())
                .unwrap();
            cfg.unconditional_edge(block3_index, block1_index).unwrap(); // loop

            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block1_index).unwrap();
            cfg.set_exit(block4_index).unwrap();

            cfg
        };

        // When: Unwind with k=1
        let unwinder = LoopUnwindingBuilder::default()
            .default_unwinding_bound(1)
            .unwinding_bound(BTreeMap::default())
            .unwinding_guard(UnwindingGuard::Assumption)
            .build()
            .unwrap();

        let mut unwound_cfg = given_cfg;
        unwinder.unwind_cfg(&mut unwound_cfg).unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block1_index = add_block_with_id(&mut cfg, "c1");
            let block2_index = add_block_with_id(&mut cfg, "c2");
            let block3_index = add_block_with_id(&mut cfg, "c3");
            let block4_index = add_block_with_id(&mut cfg, "c4");
            let block5_index = add_block_with_id(&mut cfg, "c1");
            let block6_index = add_block_with_id_and_assumption(&mut cfg, "c2", not_l.clone());

            cfg.unconditional_edge(block1_index, block2_index).unwrap();
            cfg.conditional_edge(block2_index, block4_index, not_l.clone())
                .unwrap();
            cfg.conditional_edge(block2_index, block3_index, l).unwrap();
            cfg.unconditional_edge(block3_index, block5_index).unwrap();
            cfg.unconditional_edge(block5_index, block6_index).unwrap();
            cfg.conditional_edge(block6_index, block4_index, not_l)
                .unwrap();

            cfg.block_mut(block1_index)
                .unwrap()
                .set_loop_id(Some(block1_index));
            cfg.block_mut(block5_index)
                .unwrap()
                .set_loop_id(Some(block1_index));

            cfg.set_entry(block1_index).unwrap();
            cfg.set_exit(block4_index).unwrap();

            cfg
        };

        /*debug_cfg(
            "test_unwind_loop_1_2_3_with_loop_entry_1_and_loop_exit_2_one_time",
            &given_cfg,
            &unwound_cfg,
            &expected_cfg,
        );*/

        assert_eq!(expected_cfg, unwound_cfg);
    }
}
