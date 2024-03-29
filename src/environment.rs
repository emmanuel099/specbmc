use crate::error::Result;
use serde::{Deserialize, Serialize};

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub const SPECULATION_WINDOW_SIZE: usize = 10;
pub const WORD_SIZE: usize = 64;

pub const STACK_POINTER: &str = "rsp";
pub const BASE_POINTER: &str = "rbp";

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum OptimizationLevel {
    #[serde(rename = "none")]
    Disabled,
    #[serde(rename = "basic")]
    Basic,
    #[serde(rename = "full")]
    Full,
}

impl Default for OptimizationLevel {
    fn default() -> Self {
        Self::Full
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Solver {
    #[serde(rename = "z3")]
    Z3,
    #[serde(rename = "cvc4")]
    CVC4,
    #[serde(rename = "yices2")]
    Yices2,
}

impl Default for Solver {
    fn default() -> Self {
        Self::Yices2
    }
}

impl fmt::Display for Solver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Z3 => write!(f, "Z3"),
            Self::CVC4 => write!(f, "CVC4"),
            Self::Yices2 => write!(f, "Yices2"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Check {
    #[serde(rename = "only_transient_leaks")]
    OnlyTransientExecutionLeaks,
    #[serde(rename = "only_normal_leaks")]
    OnlyNormalExecutionLeaks,
    #[serde(rename = "all_leaks")]
    AllLeaks,
}

impl Default for Check {
    fn default() -> Self {
        Self::OnlyTransientExecutionLeaks
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum PredictorStrategy {
    #[serde(rename = "choose_path")]
    ChoosePath, // aka Taken/Not-Taken Predictor
    #[serde(rename = "invert_condition")]
    InvertCondition, // aka Mis-Predict Predictor
}

impl Default for PredictorStrategy {
    fn default() -> Self {
        Self::ChoosePath
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum UnwindingGuard {
    #[serde(rename = "assumption")]
    Assumption, // add unwinding assumptions
    #[serde(rename = "assertion")]
    Assertion, // add unwinding assertions
}

impl Default for UnwindingGuard {
    fn default() -> Self {
        Self::Assumption
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Observe {
    #[serde(rename = "sequential")]
    Sequential, // Sequential with intermediate resolve edges
    #[serde(rename = "parallel")]
    Parallel, // Parallel without intermediate resolve edges (faster than full; may miss some control-flow leaks)
    #[serde(rename = "full")]
    Full, // Same as parallel but with intermediate resolve edges
    #[serde(rename = "trace")]
    Trace, // Same as parallel but full trace instead of individual observations
}

impl Default for Observe {
    fn default() -> Self {
        Self::Parallel
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Model {
    #[serde(rename = "components")]
    Components, // Components model: Observe Observe microarchitectual components like cache, branch-target buffer, ...
    #[serde(rename = "pc")]
    ProgramCounter, // PC model: Observe program counter and memory loads
}

impl Default for Model {
    fn default() -> Self {
        Self::Components
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Analysis {
    #[serde(default = "enabled")]
    pub spectre_pht: bool,
    #[serde(default = "disabled")]
    pub spectre_stl: bool,
    #[serde(default)]
    pub check: Check,
    #[serde(default)]
    pub predictor_strategy: PredictorStrategy,
    /// Default loop unwinding bound
    #[serde(default)]
    pub unwind: usize,
    /// Unwinding bounds for specific loops
    #[serde(default)]
    pub unwind_loop: BTreeMap<usize, usize>,
    #[serde(default)]
    pub unwinding_guard: UnwindingGuard,
    #[serde(default)]
    pub recursion_limit: usize,
    #[serde(default = "disabled")]
    pub start_with_empty_cache: bool,
    #[serde(default)]
    pub observe: Observe,
    #[serde(default)]
    pub model: Model,
    #[serde(default)]
    pub program_entry: Option<String>,
    #[serde(default)]
    pub inline_ignore: HashSet<String>,
}

impl Default for Analysis {
    fn default() -> Self {
        Self {
            spectre_pht: true,
            spectre_stl: false,
            check: Check::default(),
            predictor_strategy: PredictorStrategy::default(),
            unwind: 0,
            unwind_loop: BTreeMap::default(),
            unwinding_guard: UnwindingGuard::default(),
            recursion_limit: 0,
            start_with_empty_cache: false,
            observe: Observe::default(),
            model: Model::default(),
            program_entry: None,
            inline_ignore: HashSet::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Architecture {
    #[serde(default = "enabled")]
    pub cache: bool,
    #[serde(rename = "btb", default = "enabled")]
    pub branch_target_buffer: bool,
    #[serde(rename = "pht", default = "enabled")]
    pub pattern_history_table: bool,
    #[serde(default = "default_speculation_window")]
    pub speculation_window: usize,
}

impl Default for Architecture {
    fn default() -> Self {
        Self {
            cache: true,
            branch_target_buffer: true,
            pattern_history_table: true,
            speculation_window: default_speculation_window(),
        }
    }
}

pub type Address = u64;

/// A (half-open) range bounded inclusively below and exclusively above (start..end).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AddressRange {
    start: Address,
    end: Address,
}

impl AddressRange {
    pub fn new(start: Address, end: Address) -> Self {
        Self { start, end }
    }

    pub fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    pub fn addresses(&self) -> impl Iterator<Item = Address> {
        self.start..self.end
    }
}

impl Default for AddressRange {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum SecurityLevel {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "high")]
    High,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenericSecurityPolicy<T: Eq + std::hash::Hash> {
    #[serde(rename = "default")]
    pub default_level: SecurityLevel,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub low: HashSet<T>,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub high: HashSet<T>,
}

pub type RegistersSecurityPolicy = GenericSecurityPolicy<String>;
pub type MemorySecurityPolicy = GenericSecurityPolicy<AddressRange>;

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub registers: RegistersSecurityPolicy,
    pub memory: MemorySecurityPolicy,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            registers: RegistersSecurityPolicy {
                default_level: SecurityLevel::Low,
                low: HashSet::default(),
                high: HashSet::default(),
            },
            memory: MemorySecurityPolicy {
                default_level: SecurityLevel::High,
                low: HashSet::default(),
                high: HashSet::default(),
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Setup {
    #[serde(default = "disabled")]
    pub init_stack: bool,
    #[serde(rename = "registers", default)]
    pub register_content: HashMap<String, u64>,
    #[serde(rename = "flags", default)]
    pub flag_content: HashMap<String, bool>,
    #[serde(rename = "memory", default)]
    pub memory_content: HashMap<u64, Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Environment {
    #[serde(rename = "optimization", default)]
    pub optimization_level: OptimizationLevel,
    #[serde(default)]
    pub solver: Solver,
    #[serde(default)]
    pub analysis: Analysis,
    #[serde(default)]
    pub architecture: Architecture,
    #[serde(default)]
    pub policy: SecurityPolicy,
    #[serde(default)]
    pub setup: Setup,
    #[serde(default = "disabled")]
    pub debug: bool,
    #[serde(default = "enabled")]
    pub generate_counterexample: bool,
}

impl Environment {
    pub fn from_file(path: &Path) -> Result<Environment> {
        let file = File::open(path)
            .map_err(|_| format!("Environment file '{}' could not be loaded", path.display()))?;
        let reader = BufReader::new(file);
        Ok(serde_yaml::from_reader(reader)?)
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            optimization_level: OptimizationLevel::default(),
            solver: Solver::default(),
            analysis: Analysis::default(),
            architecture: Architecture::default(),
            policy: SecurityPolicy::default(),
            setup: Setup::default(),
            debug: false,
            generate_counterexample: true,
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_yaml::to_string(self).unwrap())
    }
}

fn disabled() -> bool {
    false
}

fn enabled() -> bool {
    true
}

fn default_speculation_window() -> usize {
    100
}
