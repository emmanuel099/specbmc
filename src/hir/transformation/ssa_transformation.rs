//! Static Single Assignment (SSA) Transformation

use crate::error::*;
use crate::expr;
use crate::hir::analysis;
use crate::hir::{Block, ControlFlowGraph, Instruction, PhiNode, Program};
use crate::util::Transform;
use falcon::graph::*;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Default, Builder, Debug)]
pub struct SSATransformation {}

impl Transform<Program> for SSATransformation {
    fn name(&self) -> &'static str {
        "SSATransformation"
    }

    fn description(&self) -> &'static str {
        "Transform into static single assignment form"
    }

    /// Transform the HIR program into SSA form.
    fn transform(&self, program: &mut Program) -> Result<()> {
        insert_phi_nodes(program)?;
        rename_variables(program)?;

        Ok(())
    }
}

/// Inserts phi nodes where necessary.
///
/// Implements the algorithm for constructing Semi-Pruned SSA form,
/// see Algorithm 3.1 in "SSA-based Compiler Design" book for more details.
fn insert_phi_nodes(program: &mut Program) -> Result<()> {
    let cfg = program.control_flow_graph();
    let entry = cfg.entry()?;

    if !cfg.predecessor_indices(entry)?.is_empty() {
        return Err("The CFG must not have any predecessors".into());
    }

    let dominance_frontiers = cfg.graph().compute_dominance_frontiers(entry)?;
    let global_variables = analysis::global_variables(&program);

    for (variable, defs) in variables_mutated_in_blocks(cfg) {
        if !global_variables.contains(&variable) {
            continue; // ignore local variables
        }

        let mut phi_insertions: HashSet<usize> = HashSet::new();
        let mut queue: VecDeque<usize> = defs.iter().cloned().collect();
        while let Some(block_index) = queue.pop_front() {
            for df_index in &dominance_frontiers[&block_index] {
                if phi_insertions.contains(df_index) {
                    continue;
                }

                let phi_node = {
                    let mut phi_node = PhiNode::new(variable.clone());

                    let cfg = program.control_flow_graph();
                    let df_block = cfg.block(*df_index).unwrap();

                    for predecessor in cfg.predecessor_indices(*df_index)? {
                        let pred_block = cfg.block(predecessor).unwrap();

                        // Skip rollback phi inputs (edges from transient- to default-execution) for
                        // variables which don't survive the rollback, such as memory or registers.
                        let is_rollback = pred_block.is_transient() && !df_block.is_transient();
                        if is_rollback && !variable.sort().is_rollback_persistent() {
                            continue;
                        }

                        phi_node.add_incoming(variable.clone(), predecessor);
                    }

                    phi_node
                };

                let cfg = program.control_flow_graph_mut();
                let df_block = cfg.block_mut(*df_index)?;
                df_block.add_phi_node(phi_node);

                phi_insertions.insert(*df_index);

                if !defs.contains(df_index) {
                    queue.push_back(*df_index);
                }
            }
        }
    }

    Ok(())
}

/// Get the set of variables which are mutated in the given block.
fn variables_mutated_in_block(block: &Block) -> HashSet<&expr::Variable> {
    block
        .instructions()
        .iter()
        .flat_map(Instruction::variables_written)
        .collect()
}

/// Get a mapping from variables to a set of blocks (indices) in which they are mutated.
fn variables_mutated_in_blocks(cfg: &ControlFlowGraph) -> HashMap<expr::Variable, HashSet<usize>> {
    let mut mutated_in = HashMap::new();

    for block in cfg.blocks() {
        for variable in variables_mutated_in_block(block) {
            if !mutated_in.contains_key(variable) {
                mutated_in.insert(variable.clone(), HashSet::new());
            }
            mutated_in.get_mut(variable).unwrap().insert(block.index());
        }
    }

    mutated_in
}

fn rename_variables(program: &mut Program) -> Result<()> {
    let mut versioning = VariableVersioning::new();
    program.rename_variables(&mut versioning)
}

struct VariableVersioning {
    counter: HashMap<String, usize>,
    scoped_versions: Vec<HashMap<String, usize>>,
}

impl VariableVersioning {
    pub fn new() -> Self {
        Self {
            counter: HashMap::new(),
            scoped_versions: Vec::new(),
        }
    }

    pub fn start_new_scope(&mut self) {
        let scope = match self.scoped_versions.last() {
            Some(parent_scope) => parent_scope.clone(),
            None => HashMap::new(),
        };
        self.scoped_versions.push(scope);
    }

    pub fn end_scope(&mut self) {
        self.scoped_versions.pop();
    }

    fn get_version(&mut self, variable: &expr::Variable) -> Option<usize> {
        self.scoped_versions
            .last()
            .and_then(|versions| versions.get(variable.name()))
            .copied()
    }

    fn new_version(&mut self, variable: &expr::Variable) -> Option<usize> {
        let count = self.counter.entry(variable.name().to_string()).or_insert(1);
        let version = *count;
        *count += 1;

        let versions = self.scoped_versions.last_mut().unwrap();
        versions.insert(variable.name().to_string(), version);

        Some(version)
    }
}

trait SSARename {
    fn rename_variables(&mut self, versioning: &mut VariableVersioning) -> Result<()>;
}

impl SSARename for expr::Expression {
    fn rename_variables(&mut self, versioning: &mut VariableVersioning) -> Result<()> {
        for variable in self.variables_mut() {
            variable.set_version(versioning.get_version(variable));
        }

        Ok(())
    }
}

impl SSARename for Instruction {
    fn rename_variables(&mut self, versioning: &mut VariableVersioning) -> Result<()> {
        // rename all read variables
        for variable in self.variables_read_mut() {
            variable.set_version(versioning.get_version(variable));
        }

        // introduce new SSA names for written variables
        for variable in self.variables_written_mut() {
            variable.set_version(versioning.new_version(variable));
        }

        Ok(())
    }
}

impl SSARename for Block {
    fn rename_variables(&mut self, versioning: &mut VariableVersioning) -> Result<()> {
        // introduce new SSA names for phi node outputs
        for phi_node in self.phi_nodes_mut() {
            let variable = phi_node.out_mut();
            variable.set_version(versioning.new_version(variable));
        }

        for inst in self.instructions_mut() {
            inst.rename_variables(versioning)?;
        }

        Ok(())
    }
}

impl SSARename for ControlFlowGraph {
    fn rename_variables(&mut self, versioning: &mut VariableVersioning) -> Result<()> {
        let entry = self.entry()?;

        type DominatorTree = Graph<NullVertex, NullEdge>;
        let dominator_tree = self.graph().compute_dominator_tree(entry)?;

        fn dominator_tree_dfs_pre_order_traverse(
            cfg: &mut ControlFlowGraph,
            dominator_tree: &DominatorTree,
            node: usize,
            versioning: &mut VariableVersioning,
        ) -> Result<()> {
            versioning.start_new_scope();

            let block = cfg.block_mut(node)?;
            block.rename_variables(versioning)?;

            let immediate_successors = cfg.successor_indices(node)?;
            for successor_index in immediate_successors {
                // rename variables in conditions of all outgoing edges
                let edge = cfg.edge_mut(node, successor_index)?;
                if let Some(condition) = edge.condition_mut() {
                    condition.rename_variables(versioning)?
                }

                // rename all variables of successor phi nodes which originate from this block
                let successor_block = cfg.block_mut(successor_index)?;
                for phi_node in successor_block.phi_nodes_mut() {
                    if let Some(incoming_variable) = phi_node.incoming_variable_mut(node) {
                        incoming_variable.set_version(versioning.get_version(incoming_variable));
                    }
                }
            }

            for successor in dominator_tree.successors(node)? {
                dominator_tree_dfs_pre_order_traverse(
                    cfg,
                    &dominator_tree,
                    successor.index(),
                    versioning,
                )?;
            }

            versioning.end_scope();

            Ok(())
        }

        dominator_tree_dfs_pre_order_traverse(self, &dominator_tree, entry, versioning)
    }
}

impl SSARename for Program {
    fn rename_variables(&mut self, versioning: &mut VariableVersioning) -> Result<()> {
        self.control_flow_graph_mut().rename_variables(versioning)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::Operation;

    fn expr_const(value: u64) -> expr::Expression {
        expr::BitVector::constant_u64(value, 64)
    }

    fn variable(name: &str) -> expr::Variable {
        expr::BitVector::variable(name, 64)
    }

    fn variable_ssa(name: &str, version: usize) -> expr::Variable {
        let mut variable = variable(name);
        variable.set_version(Some(version));
        variable
    }

    #[test]
    fn test_variables_mutated_in_block() {
        let block = {
            let mut block = Block::new(0);
            block.assign(variable("x"), expr_const(1)).unwrap();
            block.load(variable("y"), variable("z").into()).unwrap();
            block.assign(variable("x"), variable("y").into()).unwrap();
            block
        };

        assert_eq!(
            variables_mutated_in_block(&block),
            vec![&variable("x"), &variable("y")].into_iter().collect()
        );
    }

    #[test]
    fn test_variables_mutated_in_blocks() {
        let cfg = {
            let mut cfg = ControlFlowGraph::new();

            let block0 = cfg.new_block().unwrap();
            block0.assign(variable("x"), expr_const(1)).unwrap();

            let block1 = cfg.new_block().unwrap();
            block1.load(variable("y"), variable("z").into()).unwrap();

            let block2 = cfg.new_block().unwrap();
            block2.assign(variable("x"), variable("y").into()).unwrap();

            cfg
        };

        let mutated_in_blocks = variables_mutated_in_blocks(&cfg);

        assert_eq!(
            mutated_in_blocks[&variable("x")],
            vec![0, 2].into_iter().collect()
        );
        assert_eq!(
            mutated_in_blocks[&variable("y")],
            vec![1].into_iter().collect()
        );
    }

    #[test]
    fn test_renaming_of_expression() {
        // Given: x + y * x
        let mut expression = expr::BitVector::add(
            variable("x").into(),
            expr::BitVector::mul(variable("y").into(), variable("x").into()).unwrap(),
        )
        .unwrap();

        let mut versioning = VariableVersioning::new();
        versioning.start_new_scope();
        versioning.new_version(&variable("x")).unwrap();
        versioning.new_version(&variable("y")).unwrap();
        expression.rename_variables(&mut versioning).unwrap();

        // Expected: x_1 + y_1 * x_1
        assert_eq!(
            expression,
            expr::BitVector::add(
                variable_ssa("x", 1).into(),
                expr::BitVector::mul(variable_ssa("y", 1).into(), variable_ssa("x", 1).into())
                    .unwrap(),
            )
            .unwrap()
        );
    }

    #[test]
    fn test_renaming_of_barrier_instruction() {
        // Given: barrier
        let mut instruction = Instruction::barrier();

        let mut versioning = VariableVersioning::new();
        versioning.start_new_scope();
        instruction.rename_variables(&mut versioning).unwrap();

        // Expected: barrier
        assert_eq!(instruction, Instruction::barrier());
    }

    #[test]
    fn test_renaming_of_assign_instruction() {
        // Given: x := x
        let mut instruction = Instruction::assign(variable("x"), variable("x").into()).unwrap();

        let mut versioning = VariableVersioning::new();
        versioning.start_new_scope();
        versioning.new_version(&variable("x")).unwrap();
        instruction.rename_variables(&mut versioning).unwrap();

        // Expected: x_2 := x_1
        assert_eq!(
            instruction,
            Instruction::assign(variable_ssa("x", 2), variable_ssa("x", 1).into(),).unwrap()
        );
    }

    #[test]
    fn test_renaming_of_load_instruction() {
        // Given: x := load(x)
        let mut instruction = Instruction::load(variable("x"), variable("x").into()).unwrap();

        let mut versioning = VariableVersioning::new();
        versioning.start_new_scope();
        versioning.new_version(&variable("x")).unwrap();
        instruction.rename_variables(&mut versioning).unwrap();

        // Expected: x_2 := load(x_1)
        assert_eq!(
            instruction.operations()[0],
            Operation::Load {
                variable: variable_ssa("x", 2),
                address: variable_ssa("x", 1).into(),
            }
        );
    }

    #[test]
    fn test_renaming_of_store_instruction() {
        // Given: store(x, y)
        let mut instruction =
            Instruction::store(variable("x").into(), variable("y").into()).unwrap();

        let mut versioning = VariableVersioning::new();
        versioning.start_new_scope();
        versioning.new_version(&variable("x")).unwrap();
        versioning.new_version(&variable("y")).unwrap();
        instruction.rename_variables(&mut versioning).unwrap();

        // Expected: store(x_1, y_1)
        assert_eq!(
            instruction.operations()[0],
            Operation::Store {
                address: variable_ssa("x", 1).into(),
                expr: variable_ssa("y", 1).into(),
            }
        );
    }

    #[test]
    fn test_renaming_of_branch_instruction() {
        // Given: branch x
        let mut instruction = Instruction::branch(variable("x").into()).unwrap();

        let mut versioning = VariableVersioning::new();
        versioning.start_new_scope();
        versioning.new_version(&variable("x")).unwrap();
        instruction.rename_variables(&mut versioning).unwrap();

        // Expected: branch x_1
        assert_eq!(
            instruction,
            Instruction::branch(variable_ssa("x", 1).into()).unwrap()
        );
    }

    #[test]
    fn test_renaming_of_block() {
        // Given:
        // y = phi []
        // x = y
        // y = load(x)
        // x = y
        // z = x
        let mut block = Block::new(0);
        block.add_phi_node(PhiNode::new(variable("y")));
        block.assign(variable("x"), variable("y").into()).unwrap();
        block.load(variable("y"), variable("x").into()).unwrap();
        block.assign(variable("x"), variable("y").into()).unwrap();
        block.assign(variable("z"), variable("x").into()).unwrap();

        let mut versioning = VariableVersioning::new();
        versioning.start_new_scope();
        block.rename_variables(&mut versioning).unwrap();

        // Expected:
        // y_1 = phi []
        // x_1 = 1
        // y_2 = load(x_1)
        // x_2 = y_2
        // z_1 = x_2
        assert_eq!(
            block.phi_node(0).unwrap(),
            &PhiNode::new(variable_ssa("y", 1))
        );
        assert_eq!(
            block.instruction(0).unwrap().operations()[0],
            Operation::assign(variable_ssa("x", 1), variable_ssa("y", 1).into()).unwrap()
        );
        assert_eq!(
            block.instruction(1).unwrap().operations()[0],
            Operation::Load {
                variable: variable_ssa("y", 2),
                address: variable_ssa("x", 1).into(),
            }
        );
        assert_eq!(
            block.instruction(2).unwrap().operations()[0],
            Operation::assign(variable_ssa("x", 2), variable_ssa("y", 2).into()).unwrap()
        );
        assert_eq!(
            block.instruction(3).unwrap().operations()[0],
            Operation::assign(variable_ssa("z", 1), variable_ssa("x", 2).into()).unwrap()
        );
    }

    #[test]
    fn test_renaming_of_conditional_edges() {
        // Given:
        // x = 1
        // barr  +---+
        // -----     | (x)
        // x = x <---+
        // barr  +---+
        // -----     | (x)
        // x = x <---+
        let mut cfg = ControlFlowGraph::new();

        let block0 = cfg.new_block().unwrap();
        block0.assign(variable("x"), expr_const(1)).unwrap();
        block0.barrier();

        let block1 = cfg.new_block().unwrap();
        block1.assign(variable("x"), variable("x").into()).unwrap();
        block1.barrier();

        let block2 = cfg.new_block().unwrap();
        block2.assign(variable("x"), variable("x").into()).unwrap();

        cfg.set_entry(0).unwrap();

        cfg.conditional_edge(0, 1, variable("x").into()).unwrap();
        cfg.conditional_edge(1, 2, variable("x").into()).unwrap();

        let mut versioning = VariableVersioning::new();
        versioning.start_new_scope();
        cfg.rename_variables(&mut versioning).unwrap();

        // Expected:
        // x_1 = 1
        // barr      +---+
        // ---------     | (x_1)
        // x_2 = x_1 <---+
        // barr      +---+
        // ---------     | (x_2)
        // x_3 = x_2 <---+
        let ssa_block0 = cfg.block(0).unwrap();
        assert_eq!(
            ssa_block0.instruction(0).unwrap().operations()[0],
            Operation::assign(variable_ssa("x", 1), expr_const(1)).unwrap()
        );

        let ssa_block1 = cfg.block(1).unwrap();
        assert_eq!(
            ssa_block1.instruction(0).unwrap().operations()[0],
            Operation::assign(variable_ssa("x", 2), variable_ssa("x", 1).into()).unwrap()
        );

        let ssa_block2 = cfg.block(2).unwrap();
        assert_eq!(
            ssa_block2.instruction(0).unwrap().operations()[0],
            Operation::assign(variable_ssa("x", 3), variable_ssa("x", 2).into()).unwrap()
        );

        let ssa_edge01 = cfg.edge(0, 1).unwrap();
        assert_eq!(
            ssa_edge01.condition().unwrap(),
            &variable_ssa("x", 1).into()
        );

        let ssa_edge12 = cfg.edge(1, 2).unwrap();
        assert_eq!(
            ssa_edge12.condition().unwrap(),
            &variable_ssa("x", 2).into()
        );
    }

    #[test]
    fn test_renaming_of_incoming_edges_in_phi_nodes() {
        // Given:
        //         block 0
        //   +---+ y = 1 +---+
        //   |               |
        //   v               v
        // block 1         block 2
        // x = 2           x = 4
        // y = 3             +
        //   |               |
        //   +-------+-------+
        //           |
        //           v
        //        block 3
        // x = phi [x, 1] [x, 2]
        // y = phi [y, 1] [y, 2]
        let mut cfg = ControlFlowGraph::new();

        let block0 = cfg.new_block().unwrap();
        block0.assign(variable("y"), expr_const(1)).unwrap();

        let block1 = cfg.new_block().unwrap();
        block1.assign(variable("x"), expr_const(2)).unwrap();
        block1.assign(variable("y"), expr_const(3)).unwrap();

        let block2 = cfg.new_block().unwrap();
        block2.assign(variable("x"), expr_const(4)).unwrap();

        let mut phi_node_x = PhiNode::new(variable("x"));
        phi_node_x.add_incoming(variable("x"), 1);
        phi_node_x.add_incoming(variable("x"), 2);

        let mut phi_node_y = PhiNode::new(variable("y"));
        phi_node_y.add_incoming(variable("y"), 1);
        phi_node_y.add_incoming(variable("y"), 2);

        let block3 = cfg.new_block().unwrap();
        block3.add_phi_node(phi_node_x);
        block3.add_phi_node(phi_node_y);

        cfg.set_entry(0).unwrap();

        cfg.unconditional_edge(0, 1).unwrap();
        cfg.unconditional_edge(0, 2).unwrap();
        cfg.unconditional_edge(1, 3).unwrap();
        cfg.unconditional_edge(2, 3).unwrap();

        let mut versioning = VariableVersioning::new();
        versioning.start_new_scope();
        cfg.rename_variables(&mut versioning).unwrap();

        // Expected:
        //         block 0
        //   +---+ y_1 = 1 +---+
        //   |                 |
        //   v                 v
        // block 1           block 2
        // x_1 = 2           x_2 = 4
        // y_2 = 3             +
        //   |                 |
        //   +--------+--------+
        //            |
        //            v
        //          block 3
        // x_3 = phi [x_1, 1] [x_2, 2]
        // y_3 = phi [y_2, 1] [y_1, 2]
        let ssa_block3 = cfg.block(3).unwrap();

        let ssa_phi_node_x = ssa_block3.phi_node(0).unwrap();
        assert_eq!(ssa_phi_node_x.out(), &variable_ssa("x", 3));
        assert_eq!(
            ssa_phi_node_x.incoming_variable(1).unwrap(),
            &variable_ssa("x", 1)
        );
        assert_eq!(
            ssa_phi_node_x.incoming_variable(2).unwrap(),
            &variable_ssa("x", 2)
        );

        let ssa_phi_node_y = ssa_block3.phi_node(1).unwrap();
        assert_eq!(ssa_phi_node_y.out(), &variable_ssa("y", 3));
        assert_eq!(
            ssa_phi_node_y.incoming_variable(1).unwrap(),
            &variable_ssa("y", 2)
        );
        assert_eq!(
            ssa_phi_node_y.incoming_variable(2).unwrap(),
            &variable_ssa("y", 1)
        );
    }

    #[test]
    fn test_insert_phi_nodes() {
        // Given:
        //           block 0
        //             |
        //             v
        // +-------> block 1
        // |           |
        // |       +---+---+
        // |       |       |
        // |       v       v
        // |   block 2  block 3 +---+
        // |    x = 0      |        |
        // |       |       |        |
        // |       +---+---+        |
        // |           |            |
        // |           v            |
        // +------+ block 4         |
        //             |            |
        //             v            |
        //          block 5 <-------+
        //           y = x
        let mut program = {
            let mut cfg = ControlFlowGraph::new();

            // block0
            {
                cfg.new_block().unwrap();
            }
            // block1
            {
                cfg.new_block().unwrap();
            }
            // block2
            {
                let block = cfg.new_block().unwrap();
                block.assign(variable("x"), expr_const(0)).unwrap();
            }
            // block3
            {
                cfg.new_block().unwrap();
            }
            // block4
            {
                cfg.new_block().unwrap();
            }
            // block5
            {
                let block = cfg.new_block().unwrap();
                block.assign(variable("y"), variable("x").into()).unwrap();
            }

            cfg.unconditional_edge(0, 1).unwrap();
            cfg.unconditional_edge(1, 2).unwrap();
            cfg.unconditional_edge(1, 3).unwrap();
            cfg.unconditional_edge(2, 4).unwrap();
            cfg.unconditional_edge(3, 4).unwrap();
            cfg.unconditional_edge(3, 5).unwrap();
            cfg.unconditional_edge(4, 1).unwrap();
            cfg.unconditional_edge(4, 5).unwrap();

            cfg.set_entry(0).unwrap();

            Program::new(cfg)
        };

        insert_phi_nodes(&mut program).unwrap();

        // Expected:
        //           block 0
        //             |
        //             v
        // +-------> block 1
        // | x = phi [x, 4] [x, 0]
        // |           |
        // |       +---+---+
        // |       |       |
        // |       v       v
        // |   block 2  block 3 +---+
        // |       |       |        |
        // |       +---+---+        |
        // |           |            |
        // |           v            |
        // +------+ block 4         |
        //  x = phi [x, 2] [x, 3]   |
        //             |            |
        //             v            |
        //          block 5 <-------+
        //  x = phi [x, 4] [x, 3]
        let cfg = program.control_flow_graph();
        let block0 = cfg.block(0).unwrap();
        let block1 = cfg.block(1).unwrap();
        let block2 = cfg.block(2).unwrap();
        let block3 = cfg.block(3).unwrap();
        let block4 = cfg.block(4).unwrap();
        let block5 = cfg.block(5).unwrap();

        assert_eq!(block0.phi_nodes().len(), 0);
        assert_eq!(block1.phi_nodes().len(), 1);
        assert_eq!(block2.phi_nodes().len(), 0);
        assert_eq!(block3.phi_nodes().len(), 0);
        assert_eq!(block4.phi_nodes().len(), 1);
        assert_eq!(block5.phi_nodes().len(), 1);

        let phi_node_block1 = block1.phi_node(0).unwrap();
        assert_eq!(phi_node_block1.out(), &variable("x"));
        assert_eq!(
            phi_node_block1.incoming_variable(4).unwrap(),
            &variable("x")
        );
        assert_eq!(
            phi_node_block1.incoming_variable(0).unwrap(),
            &variable("x")
        );

        let phi_node_block4 = block4.phi_node(0).unwrap();
        assert_eq!(phi_node_block4.out(), &variable("x"));
        assert_eq!(
            phi_node_block4.incoming_variable(2).unwrap(),
            &variable("x")
        );
        assert_eq!(
            phi_node_block4.incoming_variable(3).unwrap(),
            &variable("x")
        );

        let phi_node_block5 = block5.phi_node(0).unwrap();
        assert_eq!(phi_node_block5.out(), &variable("x"));
        assert_eq!(
            phi_node_block5.incoming_variable(4).unwrap(),
            &variable("x")
        );
        assert_eq!(
            phi_node_block5.incoming_variable(3).unwrap(),
            &variable("x")
        );
    }

    #[test]
    fn test_complete_ssa_transformation() {
        // Given:
        //          block 5
        //             |
        //             v
        // +-------> block 0
        // |           |
        // |       +---+---+
        // |       |       |
        // |       v       v
        // |   block 1  block 2 +---+
        // |    x = 0   tmp = x     |
        // |       |    x = tmp     |
        // |       |       |        |
        // |       +---+---+        |
        // |           |            |
        // |           v            |
        // +------+ block 3         |
        //          x = x + x       |
        //             |            |
        //             v            |
        //          block 4 <-------+
        //           res = x
        let mut program = {
            let mut cfg = ControlFlowGraph::new();

            // block0
            {
                cfg.new_block().unwrap();
            }
            // block1
            {
                let block = cfg.new_block().unwrap();
                block.assign(variable("x"), expr_const(0)).unwrap();
            }
            // block2
            {
                let block = cfg.new_block().unwrap();
                block.assign(variable("tmp"), variable("x").into()).unwrap();
                block.assign(variable("x"), variable("tmp").into()).unwrap();
            }
            // block3
            {
                let block = cfg.new_block().unwrap();
                block
                    .assign(
                        variable("x"),
                        expr::BitVector::add(variable("x").into(), variable("x").into()).unwrap(),
                    )
                    .unwrap();
            }
            // block4
            {
                let block = cfg.new_block().unwrap();
                block.assign(variable("res"), variable("x").into()).unwrap();
            }
            // block5
            {
                cfg.new_block().unwrap();
            }

            cfg.unconditional_edge(5, 0).unwrap();
            cfg.unconditional_edge(0, 1).unwrap();
            cfg.unconditional_edge(0, 2).unwrap();
            cfg.unconditional_edge(1, 3).unwrap();
            cfg.unconditional_edge(2, 3).unwrap();
            cfg.unconditional_edge(2, 4).unwrap();
            cfg.unconditional_edge(3, 0).unwrap();
            cfg.unconditional_edge(3, 4).unwrap();

            cfg.set_entry(5).unwrap();

            Program::new(cfg)
        };

        SSATransformation::default()
            .transform(&mut program)
            .unwrap();

        // Expected:
        //           block 5
        //             |
        //             v
        // +-------> block 0
        // | x1 = phi [x5, 3] [x, 5]
        // |           |
        // |       +---+---+
        // |       |       |
        // |       v       v
        // |   block 1  block 2 +---+
        // |   x2 = 0   tmp1 = x1   |
        // |       |    x3 = tmp1   |
        // |       |       |        |
        // |       +---+---+        |
        // |           |            |
        // |           v            |
        // +------+ block 3         |
        // x4 = phi [x2, 1] [x3, 2] |
        //        x5 = x4 + x4      |
        //             |            |
        //             v            |
        //          block 4 <-------+
        //  x6 = phi [x5, 3] [x3, 2]
        //         res1 = x6
        let expected_program = {
            let mut cfg = ControlFlowGraph::new();

            // block0
            {
                let block = cfg.new_block().unwrap();
                block.add_phi_node({
                    let mut phi_node = PhiNode::new(variable_ssa("x", 1));
                    phi_node.add_incoming(variable_ssa("x", 5), 3);
                    phi_node.add_incoming(variable("x"), 5);
                    phi_node
                });
            }
            // block1
            {
                let block = cfg.new_block().unwrap();
                block.assign(variable_ssa("x", 2), expr_const(0)).unwrap();
            }
            // block2
            {
                let block = cfg.new_block().unwrap();
                block
                    .assign(variable_ssa("tmp", 1), variable_ssa("x", 1).into())
                    .unwrap();
                block
                    .assign(variable_ssa("x", 3), variable_ssa("tmp", 1).into())
                    .unwrap();
            }
            // block3
            {
                let block = cfg.new_block().unwrap();
                block
                    .assign(
                        variable_ssa("x", 5),
                        expr::BitVector::add(
                            variable_ssa("x", 4).into(),
                            variable_ssa("x", 4).into(),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                block.add_phi_node({
                    let mut phi_node = PhiNode::new(variable_ssa("x", 4));
                    phi_node.add_incoming(variable_ssa("x", 2), 1);
                    phi_node.add_incoming(variable_ssa("x", 3), 2);
                    phi_node
                });
            }
            // block4
            {
                let block = cfg.new_block().unwrap();
                block
                    .assign(variable_ssa("res", 1), variable_ssa("x", 6).into())
                    .unwrap();
                block.add_phi_node({
                    let mut phi_node = PhiNode::new(variable_ssa("x", 6));
                    phi_node.add_incoming(variable_ssa("x", 5), 3);
                    phi_node.add_incoming(variable_ssa("x", 3), 2);
                    phi_node
                });
            }
            // block5
            {
                cfg.new_block().unwrap();
            }

            cfg.unconditional_edge(5, 0).unwrap();
            cfg.unconditional_edge(0, 1).unwrap();
            cfg.unconditional_edge(0, 2).unwrap();
            cfg.unconditional_edge(1, 3).unwrap();
            cfg.unconditional_edge(2, 3).unwrap();
            cfg.unconditional_edge(2, 4).unwrap();
            cfg.unconditional_edge(3, 0).unwrap();
            cfg.unconditional_edge(3, 4).unwrap();

            cfg.set_entry(5).unwrap();

            Program::new(cfg)
        };

        assert_eq!(program, expected_program);
    }
}
