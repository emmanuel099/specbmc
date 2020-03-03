use crate::error::Result;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashSet;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

pub const SPECULATION_WINDOW_SIZE: usize = 8;
pub const WORD_SIZE: usize = 64;

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
        Self::Z3
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Analysis {
    #[serde(default = "enabled")]
    spectre_pht: bool,
    #[serde(default = "disabled")]
    spectre_stl: bool,
    #[serde(default)]
    check: Check,
    #[serde(default)]
    predictor_strategy: PredictorStrategy,
}

impl Analysis {
    pub fn spectre_pht(&self) -> bool {
        self.spectre_pht
    }

    pub fn spectre_stl(&self) -> bool {
        self.spectre_stl
    }

    pub fn check(&self) -> Check {
        self.check
    }

    pub fn predictor_strategy(&self) -> PredictorStrategy {
        self.predictor_strategy
    }
}

impl Default for Analysis {
    fn default() -> Self {
        Self {
            spectre_pht: true,
            spectre_stl: false,
            check: Check::default(),
            predictor_strategy: PredictorStrategy::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Architecture {
    #[serde(default = "enabled")]
    cache: bool,
    #[serde(rename = "btb", default = "enabled")]
    branch_target_buffer: bool,
    #[serde(rename = "pht", default = "enabled")]
    pattern_history_table: bool,
}

impl Architecture {
    pub fn cache(&self) -> bool {
        self.cache
    }

    pub fn branch_target_buffer(&self) -> bool {
        self.branch_target_buffer
    }

    pub fn pattern_history_table(&self) -> bool {
        self.pattern_history_table
    }
}

impl Default for Architecture {
    fn default() -> Self {
        Self {
            cache: true,
            branch_target_buffer: true,
            pattern_history_table: true,
        }
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
    default_level: SecurityLevel,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    low: HashSet<T>,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    high: HashSet<T>,
}

impl<T: Eq + std::hash::Hash> GenericSecurityPolicy<T> {
    pub fn default_level(&self) -> SecurityLevel {
        self.default_level
    }

    pub fn low(&self) -> &HashSet<T> {
        &self.low
    }

    pub fn high(&self) -> &HashSet<T> {
        &self.high
    }
}

pub type RegistersSecurityPolicy = GenericSecurityPolicy<String>;
pub type MemorySecurityPolicy = GenericSecurityPolicy<u64>;

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityPolicy {
    registers: RegistersSecurityPolicy,
    memory: MemorySecurityPolicy,
}

impl SecurityPolicy {
    pub fn registers(&self) -> &RegistersSecurityPolicy {
        &self.registers
    }

    pub fn memory(&self) -> &MemorySecurityPolicy {
        &self.memory
    }
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

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Environment {
    #[serde(rename = "optimization", default)]
    optimization_level: OptimizationLevel,
    #[serde(default)]
    solver: Solver,
    #[serde(default)]
    analysis: Analysis,
    #[serde(default)]
    architecture: Architecture,
    #[serde(default)]
    policy: SecurityPolicy,
    #[serde(default = "disabled")]
    debug: bool,
}

impl Environment {
    pub fn optimization_level(&self) -> OptimizationLevel {
        self.optimization_level
    }

    pub fn solver(&self) -> Solver {
        self.solver
    }

    pub fn analysis(&self) -> &Analysis {
        &self.analysis
    }

    pub fn architecture(&self) -> &Architecture {
        &self.architecture
    }

    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }

    pub fn debug(&self) -> bool {
        self.debug
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_yaml::to_string(self).unwrap())
    }
}

fn disabled() -> bool {
    false
}

fn enabled() -> bool {
    true
}

#[derive(Default)]
pub struct EnvironmentBuilder {
    file_path: Option<PathBuf>,
    optimization_level: Option<OptimizationLevel>,
    check: Option<Check>,
    solver: Option<Solver>,
    debug: Option<bool>,
}

impl EnvironmentBuilder {
    pub fn from_file(&mut self, path: &Path) -> &mut Self {
        self.file_path = Some(path.to_owned());
        self
    }

    pub fn optimization_level(&mut self, level: OptimizationLevel) -> &mut Self {
        self.optimization_level = Some(level);
        self
    }

    pub fn check(&mut self, check: Check) -> &mut Self {
        self.check = Some(check);
        self
    }

    pub fn solver(&mut self, solver: Solver) -> &mut Self {
        self.solver = Some(solver);
        self
    }

    pub fn debug(&mut self, debug: bool) -> &mut Self {
        self.debug = Some(debug);
        self
    }

    pub fn build(&mut self) -> Result<Environment> {
        let mut env = match &self.file_path {
            Some(path) => {
                let file = File::open(path)?;
                let reader = BufReader::new(file);
                serde_yaml::from_reader(reader)?
            }
            _ => Environment::default(),
        };

        if let Some(optimization_level) = self.optimization_level {
            env.optimization_level = optimization_level;
        }
        if let Some(check) = self.check {
            env.analysis.check = check;
        }
        if let Some(solver) = self.solver {
            env.solver = solver;
        }
        if let Some(debug) = self.debug {
            env.debug = debug;
        }

        Ok(env)
    }
}
