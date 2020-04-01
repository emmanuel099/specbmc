use crate::environment::Environment;
use crate::error::*;
use crate::expr::Boolean;
use crate::hir::{ControlFlowGraph, Program};
use crate::util::Transform;
use std::collections::{BTreeMap, BTreeSet};

pub struct LoopUnwinding {
    unwinding_bound: usize,
}

impl LoopUnwinding {
    pub fn new() -> Self {
        Self { unwinding_bound: 0 }
    }

    pub fn new_from_env(env: &Environment) -> Self {
        Self {
            unwinding_bound: env.analysis().unwind(),
        }
    }

    pub fn with_unwinding_bound(&mut self, bound: usize) -> &mut Self {
        self.unwinding_bound = bound;
        self
    }

    fn unwind_loop(
        &self,
        cfg: &mut ControlFlowGraph,
        loop_header: usize,
        loop_nodes: &BTreeSet<usize>,
    ) -> Result<BTreeSet<usize>> {
        // Loop unwinding adds additional nodes, collect them
        let mut loop_nodes_unwound = loop_nodes.clone();

        // Compute all back edges (edges from loop nodes to loop header) of this loop
        let back_nodes = {
            let mut nodes = Vec::new();
            for predecessor in cfg.predecessor_indices(loop_header)? {
                if loop_nodes.contains(&predecessor) {
                    nodes.push(predecessor);
                }
            }
            nodes
        };

        // Add unwinding assumption
        let unwinding_assumption = {
            let block = cfg.new_block()?;
            block.assume(Boolean::constant(false))?;
            block.index()
        };
        loop_nodes_unwound.insert(unwinding_assumption);

        // Duplicate the loop nodes
        let mut next_header = unwinding_assumption;
        for _ in 1..=self.unwinding_bound {
            let block_map = cfg.duplicate_blocks(loop_nodes)?;

            let copy_header = block_map[&loop_header];

            for back_node in &back_nodes {
                let copy_back_node = block_map[back_node];
                cfg.rewire_edge(copy_back_node, copy_header, copy_back_node, next_header)?;
            }

            for (_, block_id) in &block_map {
                loop_nodes_unwound.insert(*block_id);
            }

            next_header = copy_header;
        }

        // Now break the loops
        for back_node in &back_nodes {
            cfg.rewire_edge(*back_node, loop_header, *back_node, next_header)?;
        }

        Ok(loop_nodes_unwound)
    }

    pub fn unwind_cfg(&self, cfg: &mut ControlFlowGraph) -> Result<()> {
        let entry = cfg.entry().ok_or("CFG entry must be set")?;

        if !cfg.graph().is_reducible(entry)? {
            println!("Warning: CFG is not reducible!");
        }

        let loop_tree = cfg.graph().compute_loop_tree(entry)?;
        let parent_loop_ids = loop_tree.compute_predecessors()?;
        let loops = loop_tree.vertices();

        /*use std::fs::File;
        use std::io::Write;
        use std::path::Path;
        let mut file = File::create(Path::new("loop_tree.dot"))?;
        file.write_all(loop_tree.dot_graph().as_bytes())?;
        file.flush()?;

        println!("loops: {:?}", loops);*/

        // Initialize the set of nodes for each loop.
        // The set of nodes for each loop will grow during unwinding.
        let mut all_loop_nodes: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for l in loops {
            all_loop_nodes.insert(l.header(), l.nodes().clone());
        }

        // Unwind the loops in reverse topsort ordering, i.e. starting from the innermost loop
        let top_sort = loop_tree.compute_topological_ordering()?;
        for &loop_header in top_sort.iter().rev() {
            //println!("Unwind loop {}", loop_header);

            let loop_nodes = &all_loop_nodes[&loop_header];
            let loop_nodes_unwound = self.unwind_loop(cfg, loop_header, loop_nodes)?;

            // Now push all newly created loop nodes to the parent loops
            for &parent_loop_id in &parent_loop_ids[&loop_header] {
                all_loop_nodes
                    .entry(parent_loop_id)
                    .or_default()
                    .extend(&loop_nodes_unwound);
            }

            all_loop_nodes.insert(loop_header, loop_nodes_unwound);
        }

        Ok(())
    }
}

impl Transform<Program> for LoopUnwinding {
    fn name(&self) -> &'static str {
        "LoopUnwinding"
    }

    fn description(&self) -> &'static str {
        "Unwind loops"
    }

    fn transform(&self, program: &mut Program) -> Result<()> {
        self.unwind_cfg(program.control_flow_graph_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::expr::{Sort, Variable};
    use crate::util::RenderGraph;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    #[test]
    fn test_unwind_self_loop_zero_times() {
        // Given: Self loop at block 0
        let mut given_cfg = ControlFlowGraph::new();

        let block0_index = {
            let block = given_cfg.new_block().unwrap();
            block.barrier();
            block.index()
        };

        let block1_index = {
            let block = given_cfg.new_block().unwrap();
            block.index()
        };

        given_cfg
            .unconditional_edge(block0_index, block0_index)
            .unwrap(); // loop
        given_cfg
            .unconditional_edge(block0_index, block1_index)
            .unwrap();

        given_cfg.set_entry(block0_index).unwrap();
        given_cfg.set_exit(block1_index).unwrap();

        let given_cfg = given_cfg;

        // When: Unwind with k=0
        let mut unwound_cfg = given_cfg.clone();
        LoopUnwinding::new()
            .with_unwinding_bound(0)
            .unwind_cfg(&mut unwound_cfg)
            .unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = given_cfg.clone();

            // Unwinding assumption
            let block2_index = {
                let block = cfg.new_block().unwrap();
                block.assume(Boolean::constant(false)).unwrap();
                block.index()
            };

            // Replace back-edge
            cfg.rewire_edge(block0_index, block0_index, block0_index, block2_index)
                .unwrap();

            cfg
        };

        /*given_cfg
            .render_to_file(Path::new(
                "loop_unwinding_test_unwind_self_loop_zero_times_given.dot",
            ))
            .unwrap();
        unwound_cfg
            .render_to_file(Path::new(
                "loop_unwinding_test_unwind_self_loop_zero_times_unwound.dot",
            ))
            .unwrap();
        expected_cfg
            .render_to_file(Path::new(
                "loop_unwinding_test_unwind_self_loop_zero_times_expected.dot",
            ))
            .unwrap();*/

        assert_eq!(expected_cfg, unwound_cfg);
    }

    #[test]
    fn test_unwind_self_loop_three_times() {
        // Given: Self loop at block 0
        let mut given_cfg = ControlFlowGraph::new();

        let block0_index = {
            let block = given_cfg.new_block().unwrap();
            block.barrier();
            block.index()
        };

        let block1_index = {
            let block = given_cfg.new_block().unwrap();
            block.index()
        };

        given_cfg
            .unconditional_edge(block0_index, block0_index)
            .unwrap(); // loop
        given_cfg
            .unconditional_edge(block0_index, block1_index)
            .unwrap();

        given_cfg.set_entry(block0_index).unwrap();
        given_cfg.set_exit(block1_index).unwrap();

        let given_cfg = given_cfg;

        // When: Unwind with k=3
        let mut unwound_cfg = given_cfg.clone();
        LoopUnwinding::new()
            .with_unwinding_bound(3)
            .unwind_cfg(&mut unwound_cfg)
            .unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = given_cfg.clone();

            // Unwinding assumption
            let block2_index = {
                let block = cfg.new_block().unwrap();
                block.assume(Boolean::constant(false)).unwrap();
                block.index()
            };

            let block3_index = {
                let block = cfg.new_block().unwrap();
                block.barrier();
                block.index()
            };

            let block4_index = {
                let block = cfg.new_block().unwrap();
                block.barrier();
                block.index()
            };

            let block5_index = {
                let block = cfg.new_block().unwrap();
                block.barrier();
                block.index()
            };

            cfg.unconditional_edge(block3_index, block2_index).unwrap();
            cfg.unconditional_edge(block3_index, block1_index).unwrap();

            cfg.unconditional_edge(block4_index, block3_index).unwrap();
            cfg.unconditional_edge(block4_index, block1_index).unwrap();

            cfg.unconditional_edge(block5_index, block4_index).unwrap();
            cfg.unconditional_edge(block5_index, block1_index).unwrap();

            cfg.rewire_edge(block0_index, block0_index, block0_index, block5_index)
                .unwrap();

            cfg
        };

        /*given_cfg
            .render_to_file(Path::new(
                "loop_unwinding_test_unwind_self_loop_three_times_given.dot",
            ))
            .unwrap();
        unwound_cfg
            .render_to_file(Path::new(
                "loop_unwinding_test_unwind_self_loop_three_times_unwound.dot",
            ))
            .unwrap();
        expected_cfg
            .render_to_file(Path::new(
                "loop_unwinding_test_unwind_self_loop_three_times_expected.dot",
            ))
            .unwrap();*/

        assert_eq!(expected_cfg, unwound_cfg);
    }

    #[test]
    fn test_unwind_nested_loop_one_time() {
        // Given: Self loop at block 1 and loop 0,1,2
        let mut given_cfg = ControlFlowGraph::new();

        let block0_index = {
            let block = given_cfg.new_block().unwrap();
            block
                .assign(
                    Variable::new("c0", Sort::boolean()),
                    Boolean::constant(true),
                )
                .unwrap();
            block.index()
        };

        let block1_index = {
            let block = given_cfg.new_block().unwrap();
            block
                .assign(
                    Variable::new("c1", Sort::boolean()),
                    Boolean::constant(true),
                )
                .unwrap();
            block.index()
        };

        let block2_index = {
            let block = given_cfg.new_block().unwrap();
            block
                .assign(
                    Variable::new("c2", Sort::boolean()),
                    Boolean::constant(true),
                )
                .unwrap();
            block.index()
        };

        let block3_index = {
            let block = given_cfg.new_block().unwrap();
            block
                .assign(
                    Variable::new("c3", Sort::boolean()),
                    Boolean::constant(true),
                )
                .unwrap();
            block.index()
        };

        given_cfg
            .unconditional_edge(block0_index, block1_index)
            .unwrap();
        given_cfg
            .conditional_edge(
                block1_index,
                block1_index,
                Variable::new("b", Sort::boolean()).into(),
            )
            .unwrap(); // loop
        given_cfg
            .conditional_edge(
                block1_index,
                block2_index,
                Boolean::not(Variable::new("b", Sort::boolean()).into()).unwrap(),
            )
            .unwrap();
        given_cfg
            .conditional_edge(
                block2_index,
                block0_index,
                Variable::new("a", Sort::boolean()).into(),
            )
            .unwrap(); // loop
        given_cfg
            .conditional_edge(
                block2_index,
                block3_index,
                Boolean::not(Variable::new("a", Sort::boolean()).into()).unwrap(),
            )
            .unwrap();

        given_cfg.set_entry(block0_index).unwrap();
        given_cfg.set_exit(block3_index).unwrap();

        let given_cfg = given_cfg;

        // When: Unwind with k=1
        let mut unwound_cfg = given_cfg.clone();
        LoopUnwinding::new()
            .with_unwinding_bound(1)
            .unwind_cfg(&mut unwound_cfg)
            .unwrap();

        // Then:
        let expected_cfg = {
            let mut cfg = given_cfg.clone();

            // Unwinding assumption (loop 1)
            let block4_index = {
                let block = cfg.new_block().unwrap();
                block.assume(Boolean::constant(false)).unwrap();
                block.index()
            };

            let block5_index = {
                let block = cfg.new_block().unwrap();
                block
                    .assign(
                        Variable::new("c1", Sort::boolean()),
                        Boolean::constant(true),
                    )
                    .unwrap();
                block.index()
            };

            // Unwinding assumption (loop 0)
            let block6_index = {
                let block = cfg.new_block().unwrap();
                block.assume(Boolean::constant(false)).unwrap();
                block.index()
            };

            let block7_index = {
                let block = cfg.new_block().unwrap();
                block
                    .assign(
                        Variable::new("c0", Sort::boolean()),
                        Boolean::constant(true),
                    )
                    .unwrap();
                block.index()
            };

            let block8_index = {
                let block = cfg.new_block().unwrap();
                block
                    .assign(
                        Variable::new("c1", Sort::boolean()),
                        Boolean::constant(true),
                    )
                    .unwrap();
                block.index()
            };

            let block9_index = {
                let block = cfg.new_block().unwrap();
                block
                    .assign(
                        Variable::new("c2", Sort::boolean()),
                        Boolean::constant(true),
                    )
                    .unwrap();
                block.index()
            };

            // Unwinding assumption (loop 1)
            let block10_index = {
                let block = cfg.new_block().unwrap();
                block.assume(Boolean::constant(false)).unwrap();
                block.index()
            };

            let block11_index = {
                let block = cfg.new_block().unwrap();
                block
                    .assign(
                        Variable::new("c1", Sort::boolean()),
                        Boolean::constant(true),
                    )
                    .unwrap();
                block.index()
            };

            cfg.conditional_edge(
                block5_index,
                block4_index,
                Variable::new("b", Sort::boolean()).into(),
            )
            .unwrap();
            cfg.conditional_edge(
                block5_index,
                block2_index,
                Boolean::not(Variable::new("b", Sort::boolean()).into()).unwrap(),
            )
            .unwrap();

            cfg.rewire_edge(block1_index, block1_index, block1_index, block5_index)
                .unwrap();

            cfg.conditional_edge(
                block9_index,
                block6_index,
                Variable::new("a", Sort::boolean()).into(),
            )
            .unwrap();
            cfg.conditional_edge(
                block9_index,
                block3_index,
                Boolean::not(Variable::new("a", Sort::boolean()).into()).unwrap(),
            )
            .unwrap();

            cfg.conditional_edge(
                block11_index,
                block10_index,
                Variable::new("b", Sort::boolean()).into(),
            )
            .unwrap();
            cfg.conditional_edge(
                block11_index,
                block9_index,
                Boolean::not(Variable::new("b", Sort::boolean()).into()).unwrap(),
            )
            .unwrap();

            cfg.conditional_edge(
                block8_index,
                block11_index,
                Variable::new("b", Sort::boolean()).into(),
            )
            .unwrap();
            cfg.conditional_edge(
                block8_index,
                block9_index,
                Boolean::not(Variable::new("b", Sort::boolean()).into()).unwrap(),
            )
            .unwrap();

            cfg.unconditional_edge(block7_index, block8_index).unwrap();

            cfg.rewire_edge(block2_index, block0_index, block2_index, block7_index)
                .unwrap();

            cfg
        };

        /*given_cfg
            .render_to_file(Path::new(
                "loop_unwinding_test_unwind_nested_loop_one_time_given.dot",
            ))
            .unwrap();
        unwound_cfg
            .render_to_file(Path::new(
                "loop_unwinding_test_unwind_nested_loop_one_time_unwound.dot",
            ))
            .unwrap();
        expected_cfg
            .render_to_file(Path::new(
                "loop_unwinding_test_unwind_nested_loop_one_time_expected.dot",
            ))
            .unwrap();*/

        assert_eq!(expected_cfg, unwound_cfg);
    }
}
