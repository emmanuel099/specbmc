use crate::environment::Environment;
use crate::error::Result;
use crate::expr::{BranchTargetBuffer, Cache, Expression, PatternHistoryTable, Sort, Variable};
use crate::hir::transformation::explicit_effects::InstructionEffectEncoder;
use crate::hir::{Operation, Program};
use crate::util::Transform;

pub struct NonSpecObsEquivalence {
    cache_available: bool,
    btb_available: bool,
    pht_available: bool,
}

impl NonSpecObsEquivalence {
    pub fn new() -> Self {
        Self {
            cache_available: false,
            btb_available: false,
            pht_available: false,
        }
    }

    pub fn new_from_env(env: &Environment) -> Self {
        Self {
            cache_available: env.architecture.cache,
            btb_available: env.architecture.branch_target_buffer,
            pht_available: env.architecture.pattern_history_table,
        }
    }

    /// Enable or disable Cache effects.
    pub fn with_cache(&mut self, available: bool) -> &mut Self {
        self.cache_available = available;
        self
    }

    /// Enable or disable Branch Target Buffer effects.
    pub fn with_branch_target_buffer(&mut self, available: bool) -> &mut Self {
        self.btb_available = available;
        self
    }

    /// Enable or disable Pattern History Table effects.
    pub fn with_pattern_history_table(&mut self, available: bool) -> &mut Self {
        self.pht_available = available;
        self
    }

    fn observable_variables_nonspec(&self) -> Vec<Expression> {
        let mut variables = Vec::new();

        if self.cache_available {
            variables.push(cache_nonspec().into());
        }
        if self.btb_available {
            variables.push(btb_nonspec().into());
        }
        if self.pht_available {
            variables.push(pht_nonspec().into());
        }

        variables
    }

    fn encode_nonspec_equivalence(&self, program: &mut Program) -> Result<()> {
        let cfg = program.control_flow_graph_mut();

        // Initially low-equivalent, therefore assign same content in entry
        let entry_block = cfg.entry_block_mut()?;
        if self.cache_available {
            entry_block.assign(cache_nonspec(), Cache::variable().into())?;
        }
        if self.btb_available {
            entry_block.assign(btb_nonspec(), BranchTargetBuffer::variable().into())?;
        }
        if self.pht_available {
            entry_block.assign(pht_nonspec(), PatternHistoryTable::variable().into())?;
        }

        let encoder = InstructionEffectEncoder::new(cache_nonspec(), btb_nonspec(), pht_nonspec());

        for block in cfg
            .blocks_mut()
            .iter_mut()
            .filter(|block| !block.is_transient())
        {
            for instruction in block.instructions_mut() {
                // Collect nonspec instruction effects
                encoder.encode_effects(instruction)?;

                // Nonspec equivalent observations
                if instruction
                    .operations()
                    .iter()
                    .any(Operation::is_observable)
                {
                    instruction.add_operation(Operation::indistinguishable(
                        self.observable_variables_nonspec(),
                    ));
                }
            }
        }

        Ok(())
    }
}

fn cache_nonspec() -> Variable {
    Variable::new("_cache_ns", Sort::cache())
}

fn btb_nonspec() -> Variable {
    Variable::new("_btb_ns", Sort::branch_target_buffer())
}

fn pht_nonspec() -> Variable {
    Variable::new("_pht_ns", Sort::pattern_history_table())
}

impl Transform<Program> for NonSpecObsEquivalence {
    fn name(&self) -> &'static str {
        "NonSpecObsEquivalence"
    }

    fn description(&self) -> &'static str {
        "Add non-speculative observational equivalence constraints"
    }

    fn transform(&self, program: &mut Program) -> Result<()> {
        self.encode_nonspec_equivalence(program)?;

        Ok(())
    }
}
