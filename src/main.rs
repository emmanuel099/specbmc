#[macro_use]
extern crate clap;
use clap::Arg;
use colored::*;
use console::style;

use specbmc::environment;
use specbmc::error::Result;
use specbmc::loader;
use specbmc::solver::*;
use specbmc::util::{DumpToFile, RenderGraph, Transform, Validate};
use specbmc::{cex, hir, lir, mir};
use std::path::Path;
use std::process;

fn main() {
    let arguments = parse_arguments();
    if let Err(e) = spec_bmc(&arguments) {
        println!("{}", style(e).bold().red());
        process::exit(-1);
    }
}

struct Arguments {
    environment_file: Option<String>,
    optimization_level: Option<environment::OptimizationLevel>,
    check: Option<environment::Check>,
    solver: Option<environment::Solver>,
    predictor_strategy: Option<environment::PredictorStrategy>,
    transient_encoding_strategy: Option<environment::TransientEncodingStrategy>,
    function: Option<String>,
    unwind: Option<usize>,
    debug: bool,
    skip_solving: bool,
    cfg_file: Option<String>,
    transient_cfg_file: Option<String>,
    mir_file: Option<String>,
    lir_file: Option<String>,
    smt_file: Option<String>,
    input_file: String,
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
            Arg::with_name("transient_encoding_strategy")
                .long("trans-enc")
                .value_name("STRATEGY")
                .possible_values(&["unified", "several"])
                .help("Sets transient encoding strategy")
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
            Arg::with_name("function")
                .long("func")
                .value_name("NAME|ID")
                .help("Sets name/id of the function to be checked")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("unwind")
                .short("k")
                .long("unwind")
                .help("Unwind loops k times")
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

    let parse_transient_encoding_strategy = |strategy: &str| match strategy {
        "unified" => TransientEncodingStrategy::Unified,
        "several" => TransientEncodingStrategy::Several,
        _ => panic!("unknown transient encoding strategy"),
    };

    let parse_solver = |solver: &str| match solver {
        "z3" => Solver::Z3,
        "cvc4" => Solver::CVC4,
        "yices2" => Solver::Yices2,
        _ => panic!("unknown solver"),
    };

    return Arguments {
        environment_file: matches.value_of("environment_file").map(String::from),
        optimization_level: matches
            .value_of("optimization_level")
            .map(parse_optimization_level),
        check: matches.value_of("check").map(parse_check),
        solver: matches.value_of("solver").map(parse_solver),
        predictor_strategy: matches
            .value_of("predictor_strategy")
            .map(parse_predictory_strategy),
        transient_encoding_strategy: matches
            .value_of("transient_encoding_strategy")
            .map(parse_transient_encoding_strategy),
        function: matches.value_of("function").map(String::from),
        unwind: matches
            .value_of("unwind")
            .map(|v| v.parse::<usize>().unwrap()),
        debug: matches.is_present("debug"),
        skip_solving: matches.is_present("skip_solving"),
        cfg_file: matches.value_of("cfg_file").map(String::from),
        transient_cfg_file: matches.value_of("transient_cfg_file").map(String::from),
        mir_file: matches.value_of("mir_file").map(String::from),
        lir_file: matches.value_of("lir_file").map(String::from),
        smt_file: matches.value_of("smt_file").map(String::from),
        input_file: matches.value_of("input_file").map(String::from).unwrap(),
    };
}

fn build_environment(arguments: &Arguments) -> Result<environment::Environment> {
    use environment::*;

    let mut env_builder = EnvironmentBuilder::default();

    if let Some(file_path) = &arguments.environment_file {
        // Load given environment file
        let env_file = Path::new(file_path);
        if !env_file.is_file() {
            return Err(format!("Environment file '{}' does not exist", file_path).into());
        }
        env_builder.from_file(Path::new(env_file));
    } else {
        // Try to find a environment file for the current input
        let input_file = Path::new(&arguments.input_file);
        let env_file = input_file.with_extension("yaml");
        if env_file.is_file() {
            // Environment file exists, use it
            println!(
                "Using environment defined in '{}'",
                style(&env_file.to_str().unwrap()).yellow()
            );
            env_builder.from_file(&env_file);
        }
    }

    if let Some(level) = arguments.optimization_level {
        env_builder.optimization_level(level);
    }

    if let Some(check) = arguments.check {
        env_builder.check(check);
    }

    if let Some(strategy) = arguments.predictor_strategy {
        env_builder.predictor_strategy(strategy);
    }

    if let Some(strategy) = arguments.transient_encoding_strategy {
        env_builder.transient_encoding_strategy(strategy);
    }

    if let Some(solver) = arguments.solver {
        env_builder.solver(solver);
    }

    if let Some(unwind) = arguments.unwind {
        env_builder.unwind(unwind);
    }

    if arguments.debug {
        env_builder.debug(true);
    }

    env_builder.build()
}

fn hir_transformations(env: &environment::Environment, program: &mut hir::Program) -> Result<()> {
    use hir::transformation::*;

    let transformations = {
        let mut steps: Vec<Box<dyn Transform<hir::Program>>> = Vec::new();
        steps.push(Box::new(LoopUnwinding::new_from_env(env)));
        steps.push(Box::new(InstructionEffects::new_from_env(env)));
        if env.analysis().check() != environment::Check::OnlyNormalExecutionLeaks {
            steps.push(Box::new(TransientExecution::new_from_env(env)));
        }
        steps.push(Box::new(InitGlobalVariables::new_from_env(env)));
        steps.push(Box::new(Observations::new_from_env(env)));
        steps.push(Box::new(ExplicitEffects::new()));
        steps.push(Box::new(ExplicitMemory::new()));
        if env.analysis().check() == environment::Check::OnlyTransientExecutionLeaks {
            steps.push(Box::new(NonSpecObsEquivalence::new_from_env(env)));
        }
        steps.push(Box::new(SSATransformation::new()));
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

        if env.debug() {
            program
                .control_flow_graph()
                .render_to_file(Path::new(&format!("dbg_hir_{}.dot", transformation.name())))?;
        }
    }

    Ok(())
}

fn lir_optimize(env: &environment::Environment, program: &mut lir::Program) -> Result<()> {
    use lir::optimization::*;

    let optimizer = match env.optimization_level() {
        environment::OptimizationLevel::Disabled => Optimizer::none(),
        environment::OptimizationLevel::Basic => Optimizer::basic(),
        environment::OptimizationLevel::Full => Optimizer::full(),
    };

    optimizer.optimize(program)?;

    Ok(())
}

fn spec_bmc(arguments: &Arguments) -> Result<()> {
    let input_file = &arguments.input_file;

    let env = build_environment(arguments)?;

    if env.debug() {
        println!("{}:\n{}\n---", "Environment".bold(), style(&env).cyan());
    }

    println!(
        "{} Loading program '{}'",
        style("[1/9]").bold().dim(),
        input_file.yellow()
    );
    let mut hir_program =
        loader::load_program(Path::new(input_file), arguments.function.as_deref())?;

    if let Some(path) = &arguments.cfg_file {
        hir_program
            .control_flow_graph()
            .render_to_file(Path::new(path))?;
    }

    println!("{} Transforming HIR ...", style("[4/9]").bold().dim());
    hir_transformations(&env, &mut hir_program)?;

    if let Some(path) = &arguments.transient_cfg_file {
        hir_program
            .control_flow_graph()
            .render_to_file(Path::new(path))?;
    }

    println!("{} Translating into MIR", style("[5/9]").bold().dim());
    let mir_program = mir::Program::from(&hir_program)?;

    if let Some(path) = &arguments.mir_file {
        mir_program.block_graph().render_to_file(Path::new(path))?;
    }

    println!("{} Translating into LIR", style("[6/9]").bold().dim());
    let mut lir_program = lir::Program::from(&mir_program)?;
    lir_program.validate()?;

    println!("{} Optimizing LIR", style("[7/9]").bold().dim());
    lir_optimize(&env, &mut lir_program)?;

    if let Some(path) = &arguments.lir_file {
        lir_program.dump_to_file(Path::new(path))?;
    }

    let mut solver = create_solver(&env)?;
    if let Some(path) = &arguments.smt_file {
        solver.dump_formula_to_file(Path::new(path))?
    }

    println!("{} Encoding LIR", style("[8/9]").bold().dim());
    solver.encode_program(&lir_program)?;

    if arguments.skip_solving {
        return Ok(());
    }

    println!("{} Searching for leaks ...", style("[9/9]").bold().dim());
    match solver.check_assertions()? {
        CheckResult::AssertionsHold => {
            println!("{}", "Program is safe.".bold().green());
        }
        CheckResult::AssertionViolated { model } => {
            println!("{}", "Leak detected!".bold().red());

            let counter_example = cex::build_counter_example(&hir_program, &model)?;
            counter_example
                .control_flow_graph()
                .render_to_file(Path::new("cex.dot"))?;

            process::exit(1);
        }
    }

    Ok(())
}
