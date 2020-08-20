#[macro_use]
extern crate clap;
use clap::Arg;
use colored::*;
use console::style;

use specbmc::environment;
use specbmc::error::Result;
use specbmc::ir::{Transform, TryTranslateFrom, Validate};
use specbmc::loader;
use specbmc::solver::*;
use specbmc::util::{DumpToFile, RenderGraph};
use specbmc::{cex, hir, lir, mir};
use std::path::Path;
use std::process;

fn main() {
    let arguments = parse_arguments();
    if let Err(e) = spec_bmc(&arguments) {
        println!("{}", style(&e).bold().red());
        if let Some(backtrace) = e.backtrace() {
            println!("{:?}", backtrace);
        }
        process::exit(1);
    }
}

struct Arguments {
    environment_file: Option<String>,
    optimization_level: Option<environment::OptimizationLevel>,
    check: Option<environment::Check>,
    solver: Option<environment::Solver>,
    predictor_strategy: Option<environment::PredictorStrategy>,
    program_entry: Option<String>,
    unwind: Option<usize>,
    unwinding_guard: Option<environment::UnwindingGuard>,
    speculation_window: Option<usize>,
    debug: bool,
    skip_solving: bool,
    skip_cex: bool,
    cfg_file: Option<String>,
    transient_cfg_file: Option<String>,
    call_graph_file: Option<String>,
    mir_file: Option<String>,
    lir_file: Option<String>,
    smt_file: Option<String>,
    input_file: String,
    print_assembly_info: bool,
}

fn parse_arguments() -> Arguments {
    use environment::*;

    fn is_positive_number(s: String) -> std::result::Result<(), String> {
        if s.parse::<usize>().is_ok() {
            Ok(())
        } else {
            Err(format!("{} isn't a positive number", s))
        }
    }

    let matches = app_from_crate!()
        .arg(
            Arg::with_name("environment_file")
                .short("e")
                .long("env")
                .value_name("FILE")
                .help("Sets environment file to use (arguments overwrite it)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("optimization_level")
                .short("o")
                .long("opt")
                .value_name("LEVEL")
                .possible_values(&["none", "basic", "full"])
                .help("Sets optimization level")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("check")
                .short("c")
                .long("check")
                .value_name("TYPE")
                .possible_values(&["all", "normal", "transient"])
                .help("Sets leak check type")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("predictor_strategy")
                .short("p")
                .long("predictor")
                .value_name("STRATEGY")
                .possible_values(&["invert", "choose"])
                .help("Sets predictor strategy")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("solver")
                .long("solver")
                .value_name("SOLVER")
                .possible_values(&["z3", "cvc4", "yices2"])
                .help("Sets solver to use")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("program_entry")
                .long("entry")
                .value_name("NAME|ADDRESS")
                .help("Sets name/address of the program entry function")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("unwind")
                .short("k")
                .long("unwind")
                .value_name("k")
                .help("Unwind loops k times")
                .validator(is_positive_number)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("unwinding_guard")
                .long("unwinding-guard")
                .value_name("GUARD")
                .possible_values(&["assumption", "assertion"])
                .help("Sets the unwinding guard")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("speculation_window")
                .short("s")
                .long("spec-win")
                .value_name("WINDOW")
                .help("Sets the maximum length of the speculation window")
                .validator(is_positive_number)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("debug")
                .short("d")
                .long("debug")
                .help("Enables debug mode"),
        )
        .arg(
            Arg::with_name("skip_solving")
                .long("skip-solving")
                .help("Skips solving the SMT formula"),
        )
        .arg(
            Arg::with_name("skip_cex")
                .long("skip-cex")
                .help("Skips generating of counterexample"),
        )
        .arg(
            Arg::with_name("cfg_file")
                .long("cfg")
                .value_name("FILE")
                .help("Prints CFG into the file (DOT)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("transient_cfg_file")
                .long("trans-cfg")
                .value_name("FILE")
                .help("Prints CFG (with transient behavior) into the file (DOT)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("call_graph_file")
                .long("call-graph")
                .value_name("FILE")
                .help("Prints call graph into the file (DOT)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mir_file")
                .long("mir")
                .value_name("FILE")
                .help("Prints MIR program into the file (DOT)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("lir_file")
                .long("lir")
                .value_name("FILE")
                .help("Prints LIR program into the file (plain text)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("smt_file")
                .long("smt")
                .value_name("FILE")
                .help("Prints SMT-2 formula into the file (plain text)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("input_file")
                .value_name("FILE")
                .help("Input file to be checked")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("print_assembly_info")
                .short("a")
                .long("assembly-info")
                .help("Prints assembly info and exits"),
        )
        .get_matches();

    let parse_optimization_level = |level: &str| match level {
        "none" => OptimizationLevel::Disabled,
        "basic" => OptimizationLevel::Basic,
        "full" => OptimizationLevel::Full,
        _ => panic!("unknown optimization level"),
    };

    let parse_check = |check: &str| match check {
        "all" => Check::AllLeaks,
        "normal" => Check::OnlyNormalExecutionLeaks,
        "transient" => Check::OnlyTransientExecutionLeaks,
        _ => panic!("unknown check type"),
    };

    let parse_predictory_strategy = |strategy: &str| match strategy {
        "invert" => PredictorStrategy::InvertCondition,
        "choose" => PredictorStrategy::ChoosePath,
        _ => panic!("unknown predictor strategy"),
    };

    let parse_solver = |solver: &str| match solver {
        "z3" => Solver::Z3,
        "cvc4" => Solver::CVC4,
        "yices2" => Solver::Yices2,
        _ => panic!("unknown solver"),
    };

    let parse_unwinding_guard = |guard: &str| match guard {
        "assumption" => UnwindingGuard::Assumption,
        "assertion" => UnwindingGuard::Assertion,
        _ => panic!("unknown unwinding guard"),
    };

    Arguments {
        environment_file: matches.value_of("environment_file").map(String::from),
        optimization_level: matches
            .value_of("optimization_level")
            .map(parse_optimization_level),
        check: matches.value_of("check").map(parse_check),
        solver: matches.value_of("solver").map(parse_solver),
        predictor_strategy: matches
            .value_of("predictor_strategy")
            .map(parse_predictory_strategy),
        program_entry: matches.value_of("program_entry").map(String::from),
        unwind: matches
            .value_of("unwind")
            .map(|v| v.parse::<usize>().unwrap()),
        unwinding_guard: matches
            .value_of("unwinding_guard")
            .map(parse_unwinding_guard),
        speculation_window: matches
            .value_of("speculation_window")
            .map(|v| v.parse::<usize>().unwrap()),
        debug: matches.is_present("debug"),
        skip_solving: matches.is_present("skip_solving"),
        skip_cex: matches.is_present("skip_cex"),
        cfg_file: matches.value_of("cfg_file").map(String::from),
        transient_cfg_file: matches.value_of("transient_cfg_file").map(String::from),
        call_graph_file: matches.value_of("call_graph_file").map(String::from),
        mir_file: matches.value_of("mir_file").map(String::from),
        lir_file: matches.value_of("lir_file").map(String::from),
        smt_file: matches.value_of("smt_file").map(String::from),
        input_file: matches.value_of("input_file").map(String::from).unwrap(),
        print_assembly_info: matches.is_present("print_assembly_info"),
    }
}

fn build_environment(arguments: &Arguments) -> Result<environment::Environment> {
    use environment::*;

    let mut env = if let Some(file_path) = &arguments.environment_file {
        // Load given environment file
        Environment::from_file(Path::new(file_path))?
    } else {
        // Try to find a environment file for the current input and use it if it exists
        let input_file = Path::new(&arguments.input_file);
        let env_file = input_file.with_extension("yaml");
        match Environment::from_file(&env_file) {
            Ok(env) => {
                println!(
                    "Using environment defined in '{}'.",
                    style(&env_file.to_str().unwrap()).yellow()
                );
                env
            }
            Err(_) => {
                println!("Using default environment.");
                Environment::default()
            }
        }
    };

    if let Some(level) = arguments.optimization_level {
        env.optimization_level = level;
    }

    if let Some(check) = arguments.check {
        env.analysis.check = check;
    }

    if let Some(strategy) = arguments.predictor_strategy {
        env.analysis.predictor_strategy = strategy;
    }

    if let Some(solver) = arguments.solver {
        env.solver = solver;
    }

    if let Some(unwind) = arguments.unwind {
        env.analysis.unwind = unwind;
    }

    if let Some(unwinding_guard) = arguments.unwinding_guard {
        env.analysis.unwinding_guard = unwinding_guard;
    }

    if let Some(speculation_window) = arguments.speculation_window {
        env.architecture.speculation_window = speculation_window;
    }

    if let Some(entry) = &arguments.program_entry {
        env.analysis.program_entry = Some(entry.clone());
    }

    if arguments.debug {
        env.debug = true;
    }

    Ok(env)
}

fn hir_transformations(
    env: &environment::Environment,
    program: &mut hir::InlinedProgram,
) -> Result<()> {
    use hir::transformation::*;

    let transformations = {
        let mut steps: Vec<Box<dyn Transform<hir::InlinedProgram>>> = Vec::new();
        steps.push(Box::new(LoopUnwinding::new_from_env(env)));
        steps.push(Box::new(InstructionEffects::new_from_env(env)));
        if env.analysis.check != environment::Check::OnlyNormalExecutionLeaks {
            steps.push(Box::new(TransientExecution::new_from_env(env)));
        }
        steps.push(Box::new(InitGlobalVariables::new_from_env(env)));
        steps.push(Box::new(Observations::new_from_env(env)));
        steps.push(Box::new(ExplicitEffects::default()));
        steps.push(Box::new(ExplicitMemory::default()));
        if env.analysis.check == environment::Check::OnlyTransientExecutionLeaks {
            steps.push(Box::new(NonSpecObsEquivalence::new_from_env(env)));
        }
        steps.push(Box::new(SSATransformation::new(SSAForm::Pruned)));
        steps
    };

    for (idx, transformation) in transformations.iter().enumerate() {
        println!(
            "-> {} {}",
            style(format!("[{}/{}]", idx + 1, transformations.len()))
                .bold()
                .dim(),
            transformation.description(),
        );

        transformation.transform(program)?;

        if env.debug {
            program
                .control_flow_graph()
                .render_to_file(Path::new(&format!("dbg_hir_{}.dot", transformation.name())))?;
        }
    }

    Ok(())
}

fn lir_optimize(env: &environment::Environment, program: &mut lir::Program) -> Result<()> {
    use lir::optimization::*;

    let optimizer = match env.optimization_level {
        environment::OptimizationLevel::Disabled => Optimizer::none(),
        environment::OptimizationLevel::Basic => Optimizer::basic(),
        environment::OptimizationLevel::Full => Optimizer::full(),
    };

    optimizer.optimize(program)?;

    Ok(())
}

fn spec_bmc(arguments: &Arguments) -> Result<()> {
    if arguments.print_assembly_info {
        print_assembly_info(arguments)?;
        return Ok(());
    }

    check_program(arguments)
}

fn print_assembly_info(arguments: &Arguments) -> Result<()> {
    let input_file = Path::new(&arguments.input_file);

    let input_file_path = Path::new(input_file);
    let loader = loader::loader_for_file(input_file_path).ok_or("No compatible loader found")?;

    let info = loader.assembly_info()?;
    println!("{}", info);

    Ok(())
}

fn parse_program_entry(s: &str) -> hir::ProgramEntry {
    if s.starts_with("0x") {
        let s = s.trim_start_matches("0x");
        if let Ok(addr) = u64::from_str_radix(s, 16) {
            return hir::ProgramEntry::Address(addr);
        }
    }

    hir::ProgramEntry::Name(s.to_owned())
}

fn check_program(arguments: &Arguments) -> Result<()> {
    let input_file = &arguments.input_file;

    let env = build_environment(arguments)?;

    if env.debug {
        println!("{}:\n{}\n---", "Environment".bold(), style(&env).cyan());
    }

    println!(
        "{} Load program '{}'",
        style("[1/8]").bold().dim(),
        input_file.yellow()
    );
    let input_file_path = Path::new(input_file);
    let loader = loader::loader_for_file(input_file_path).ok_or("No compatible loader found")?;
    let mut program = loader.load_program()?;

    // Overwrite program entry if set
    if let Some(entry) = &env.analysis.program_entry {
        let entry = parse_program_entry(entry);
        program.set_entry(entry)?;
    }

    println!("{} Inline functions", style("[2/8]").bold().dim());
    if let Some(path) = &arguments.call_graph_file {
        let call_graph = hir::analysis::call_graph(&program);
        call_graph.render_to_file(Path::new(path))?;
    }
    let function_inlining = hir::transformation::FunctionInlining::new();
    let mut hir_program = function_inlining.inline(&program)?;

    if let Some(path) = &arguments.cfg_file {
        hir_program
            .control_flow_graph()
            .render_to_file(Path::new(path))?;
    }

    println!("{} Transform HIR ...", style("[3/8]").bold().dim());
    hir_transformations(&env, &mut hir_program)?;

    if let Some(path) = &arguments.transient_cfg_file {
        hir_program
            .control_flow_graph()
            .render_to_file(Path::new(path))?;
    }

    println!("{} Translate into MIR", style("[4/8]").bold().dim());
    let mir_program = mir::Program::try_translate_from(&hir_program)?;

    if let Some(path) = &arguments.mir_file {
        mir_program.block_graph().render_to_file(Path::new(path))?;
    }

    println!("{} Translate into LIR", style("[5/8]").bold().dim());
    let mut lir_program = lir::Program::try_translate_from(&mir_program)?;
    lir_program.validate()?;

    println!("{} Optimize LIR", style("[6/8]").bold().dim());
    lir_optimize(&env, &mut lir_program)?;

    if let Some(path) = &arguments.lir_file {
        lir_program.dump_to_file(Path::new(path))?;
    }

    let mut solver = create_solver(&env)?;
    if let Some(path) = &arguments.smt_file {
        solver.dump_formula_to_file(Path::new(path))?
    }

    println!(
        "{} Encode LIR as SMT formula (solver={})",
        style("[7/8]").bold().dim(),
        env.solver
    );
    solver.encode_program(&lir_program)?;

    if arguments.skip_solving {
        return Ok(());
    }

    println!("{} Search for leaks ...", style("[8/8]").bold().dim());
    match solver.check_assertions()? {
        CheckResult::AssertionsHold => {
            println!("{}", "Program is safe.".bold().green());
        }
        CheckResult::AssertionViolated { model } => {
            println!("{}", "Leak detected!".bold().red());

            if !arguments.skip_cex {
                let counter_example = cex::build_counter_example(&hir_program, model.as_ref())?;
                counter_example
                    .control_flow_graph()
                    .render_to_file(Path::new("cex.dot"))?;
            }

            process::exit(2);
        }
    }

    Ok(())
}
