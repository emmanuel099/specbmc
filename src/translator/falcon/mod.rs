use crate::error::Result;
use crate::ir;
use falcon::il;

pub fn translate_function(function: &il::Function) -> Result<ir::Program> {
    let block_graph = translate_control_flow_graph(function.control_flow_graph())?;

    Ok(ir::Program::new(block_graph))
}

fn translate_control_flow_graph(cfg: &il::ControlFlowGraph) -> Result<ir::BlockGraph> {
    let mut block_graph = ir::BlockGraph::new();

    for block in cfg.blocks() {
        block_graph.add_block(translate_block(block)?)?;
    }
    for edge in cfg.edges() {
        block_graph.add_edge(edge.head(), edge.tail())?;
    }

    if let Some(entry) = cfg.entry() {
        block_graph.set_entry(entry)?;
    }

    Ok(block_graph)
}

fn translate_block(src_block: &il::Block) -> Result<ir::Block> {
    let mut block = ir::Block::new(src_block.index());

    for phi_node in src_block.phi_nodes() {
        let var = translate_scalar(phi_node.out())?;
        let expr = ir::Expression::constant(ir::Constant::new(42, 64));
        block.add_let(var, expr)?;
    }

    for instruction in src_block.instructions() {
        match instruction.operation() {
            il::Operation::Assign { ref dst, ref src } => {
                let var = translate_scalar(dst)?;
                let expr = translate_expression(src)?;
                let node = block.add_let(var, expr)?;
                node.set_address(instruction.address());
            }
            il::Operation::Store { ref index, ref src } => {
                let var = ir::Variable::new("$memory", ir::Sort::Memory);
                let addr = translate_expression(index)?;
                let node = block.add_let(var, addr)?;
                node.set_address(instruction.address());
            }
            il::Operation::Load { ref dst, ref index } => {
                let var = translate_scalar(dst)?;
                let addr = translate_expression(index)?;
                let node = block.add_let(var, addr)?;
                node.set_address(instruction.address());
            }
            il::Operation::Branch { .. } => continue,
            il::Operation::Intrinsic { .. } => continue,
            il::Operation::Nop => continue,
        }
    }

    Ok(block)
}

fn translate_expression(src_expr: &il::Expression) -> Result<ir::Expression> {
    Ok(ir::Expression::constant(ir::Constant::new(42, 64)))
}

fn translate_scalar(scalar: &il::Scalar) -> Result<ir::Variable> {
    let sort = ir::Sort::BitVector(scalar.bits());
    let mut var = ir::Variable::new(scalar.name(), sort);
    var.set_version(scalar.ssa());
    Ok(var)
}
