#[macro_use]
extern crate clap;
use clap::Arg;
use colored::*;
use console::style;

use specbmc::environment;
use specbmc::error::Result;
use specbmc::ir::{TryTranslateFrom, Validate};
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
    observe: Option<environment::Observe>,
    model: Option<environment::Model>,
    program_entry: Option<String>,
    unwind: Option<usize>,
    unwinding_guard: Option<environment::UnwindingGuard>,
    recursion_limit: Option<usize>,
    speculation_window: Option<usize>,
    debug: bool,
    skip_solving: bool,
    skip_cex: bool,
    cex_file: String,
    cfg_file: Option<String>,
    transient_cfg_file: Option<String>,
    call_graph_file: Option<String>,
    mir_file: Option<String>,
    lir_file: Option<String>,
    smt_file: Option<String>,
    input_file: String,
    print_assembly_info: bool,
    show_environment: bool,
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
            Arg::with_name("observe")
                .long("observe")
                .value_name("OBSERVE")
                .possible_values(&["sequential", "parallel", "full"])
                .help("Sets observation type")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("model")
                .long("model")
                .value_name("MODEL")
                .possible_values(&["components", "pc"])
                .help("Sets analysis model type")
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
                .help("Sets solver")
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
                .help("Unwinds loops k times")
                .validator(is_positive_number)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("unwinding_guard")
                .long("unwinding-guard")
                .value_name("GUARD")
                .possible_values(&["assumption", "assertion"])
                .help("Sets unwinding guard")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("recursion_limit")
                .short("r")
                .long("recursion")
                .value_name("LIMIT")
                .help("Inlines recursive functions at most LIMIT times")
                .validator(is_positive_number)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("speculation_window")
                .short("s")
                .long("spec-win")
                .value_name("WINDOW")
                .help("Sets maximum length of the speculation window")
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
                .help("Skips solving SMT formula"),
        )
        .arg(
            Arg::with_name("skip_cex")
                .long("skip-cex")
                .help("Skips generating counterexample"),
        )
        .arg(
            Arg::with_name("cex_file")
                .long("cex")
                .value_name("FILE")
                .help("Prints counterexample into file (DOT)")
                .default_value("cex.dot")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("cfg_file")
                .long("cfg")
                .value_name("FILE")
                .help("Prints control-flow graph into file (DOT)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("transient_cfg_file")
                .long("trans-cfg")
                .value_name("FILE")
                .help("Prints CFG (with transient behavior) into file (DOT)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("call_graph_file")
                .long("call-graph")
                .value_name("FILE")
                .help("Prints call graph into file (DOT)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mir_file")
                .long("mir")
                .value_name("FILE")
                .help("Prints MIR program into file (DOT)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("lir_file")
                .long("lir")
                .value_name("FILE")
                .help("Prints LIR program into file (plain text)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("smt_file")
                .long("smt")
                .value_name("FILE")
                .help("Prints SMT-2 formula into file (plain text)")
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
        .arg(
            Arg::with_name("show_environment")
                .long("show-env")
                .help("Prints the environment to console"),
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

    let parse_observe = |observe: &str| match observe {
        "sequential" => Observe::Sequential,
        "parallel" => Observe::Parallel,
        "full" => Observe::Full,
        _ => panic!("unknown observe type"),
    };

    let parse_model = |model: &str| match model {
        "components" => Model::Components,
        "pc" => Model::ProgramCounter,
        _ => panic!("unknown model type"),
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
        observe: matches.value_of("observe").map(parse_observe),
        model: matches.value_of("model").map(parse_model),
        program_entry: matches.value_of("program_entry").map(String::from),
        unwind: matches
            .value_of("unwind")
            .map(|v| v.parse::<usize>().unwrap()),
        unwinding_guard: matches
            .value_of("unwinding_guard")
            .map(parse_unwinding_guard),
        recursion_limit: matches
            .value_of("recursion_limit")
            .map(|v| v.parse::<usize>().unwrap()),
        speculation_window: matches
            .value_of("speculation_window")
            .map(|v| v.parse::<usize>().unwrap()),
        debug: matches.is_present("debug"),
        skip_solving: matches.is_present("skip_solving"),
        skip_cex: matches.is_present("skip_cex"),
        cex_file: matches.value_of("cex_file").map(String::from).unwrap(),
        cfg_file: matches.value_of("cfg_file").map(String::from),
        transient_cfg_file: matches.value_of("transient_cfg_file").map(String::from),
        call_graph_file: matches.value_of("call_graph_file").map(String::from),
        mir_file: matches.value_of("mir_file").map(String::from),
        lir_file: matches.value_of("lir_file").map(String::from),
        smt_file: matches.value_of("smt_file").map(String::from),
        input_file: matches.value_of("input_file").map(String::from).unwrap(),
        print_assembly_info: matches.is_present("print_assembly_info"),
        show_environment: matches.is_present("show_environment"),
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

    if let Some(observe) = arguments.observe {
        env.analysis.observe = observe;
    }

    if let Some(model) = arguments.model {
        env.analysis.model = model;
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

    if let Some(recursion_limit) = arguments.recursion_limit {
        env.analysis.recursion_limit = recursion_limit;
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

    if arguments.skip_cex {
        env.generate_counterexample = false;
    }

    Ok(env)
}

fn hir_transformations(
    env: &environment::Environment,
    program: &mut hir::InlinedProgram,
) -> Result<()> {
    let transformations = hir::transformation::create_transformations(env)?;

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

    if arguments.show_environment || env.debug {
        println!("{}:\n{}\n---", "Environment".bold(), style(&env).cyan());
    }

    let bullet_point = style(">>").bold().dim();

    println!("{} Load program '{}'", bullet_point, input_file.yellow());
    let input_file_path = Path::new(input_file);
    let loader = loader::loader_for_file(input_file_path).ok_or("No compatible loader found")?;
    let mut program = loader.load_program()?;

    // Overwrite program entry if set
    if let Some(entry) = &env.analysis.program_entry {
        let entry = parse_program_entry(entry);
        program.set_entry(entry)?;
    }

    println!("{} Inline functions", bullet_point);
    if let Some(path) = &arguments.call_graph_file {
        let call_graph = hir::analysis::call_graph(&program);
        call_graph.render_to_file(Path::new(path))?;
    }
    let function_inlining = hir::transformation::FunctionInliningBuilder::default()
        .recursion_limit(env.analysis.unwind)
        .build()?;
    let mut hir_program = function_inlining.inline(&program)?;

    if let Some(path) = &arguments.cfg_file {
        hir_program
            .control_flow_graph()
            .render_to_file(Path::new(path))?;
    }

    println!("{} Transform HIR ...", bullet_point);
    hir_transformations(&env, &mut hir_program)?;

    if let Some(path) = &arguments.transient_cfg_file {
        hir_program
            .control_flow_graph()
            .render_to_file(Path::new(path))?;
    }

    println!("{} Translate into MIR", bullet_point);
    let mir_program = mir::Program::try_translate_from(&hir_program)?;

    if let Some(path) = &arguments.mir_file {
        mir_program.block_graph().render_to_file(Path::new(path))?;
    }

    println!("{} Translate into LIR", bullet_point);
    let mut lir_program = lir::Program::try_translate_from(&mir_program)?;
    lir_program.validate()?;

    println!("{} Optimize LIR", bullet_point);
    let lir_optimizer = lir::optimization::Optimizer::new_from_env(&env);
    lir_optimizer.optimize(&mut lir_program)?;

    if let Some(path) = &arguments.lir_file {
        lir_program.dump_to_file(Path::new(path))?;
    }

    let mut solver = create_solver(&env)?;
    if let Some(path) = &arguments.smt_file {
        solver.dump_formula_to_file(Path::new(path))?
    }

    println!(
        "{} Encode LIR as SMT formula (solver={})",
        bullet_point, env.solver
    );
    solver.encode_program(&lir_program)?;

    if arguments.skip_solving {
        return Ok(());
    }

    println!("{} Search for leaks ...", bullet_point);
    match solver.check_assertions()? {
        CheckResult::AssertionsHold => {
            println!("{}", "Program is safe.".bold().green());
        }
        CheckResult::AssertionViolated { model } => {
            println!("{}", "Leak detected!".bold().red());

            if env.generate_counterexample {
                println!(
                    "{} Generate counterexample ({})",
                    bullet_point, arguments.cex_file
                );

                let counter_example = cex::build_counter_example(&hir_program, model.as_ref())?;
                counter_example
                    .control_flow_graph()
                    .render_to_file(Path::new(&arguments.cex_file))?;
            }

            process::exit(2);
        }
    }

    Ok(())
}
