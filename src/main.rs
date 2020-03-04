#[macro_use]
extern crate clap;
use clap::{Arg, ArgMatches};
use colored::*;
use console::style;
use console::Term;
use specbmc::environment;
use specbmc::error::Result;
use specbmc::loader;
use specbmc::solver::*;
use specbmc::util::{RenderGraph, Transform, Validate};
use specbmc::{hir, lir, mir};
use std::path::Path;
use std::process;

fn main() {
    let arguments = app_from_crate!()
        .arg(
            Arg::with_name("environment_file")
                .short("e")
                .long("env")
                .value_name("FILE")
                .help("Sets environment file to use")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("optimization_level")
                .short("o")
                .long("opt")
                .value_name("LEVEL")
                .possible_values(&["none", "basic", "full"])
                .help("Sets optimization level (overwrites environment)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("check")
                .short("c")
                .long("check")
                .value_name("TYPE")
                .possible_values(&["all", "normal", "transient"])
                .help("Sets leak check type (overwrites environment)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("solver")
                .long("solver")
                .value_name("SOLVER")
                .possible_values(&["z3", "cvc4", "yices2"])
                .help("Sets solver to use (overwrites environment)")
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
                .help("Prints CFG into the file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("transient_cfg_file")
                .long("trans-cfg")
                .value_name("FILE")
                .help("Prints CFG (with transient behavior) into the file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("smt_file")
                .long("smt")
                .value_name("FILE")
                .help("Prints SMT-2 formula into the file")
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

    if let Err(e) = spec_bmc(&arguments) {
        println!("{}", style(e).bold().red());
        process::exit(-1);
    }
}

fn build_environment(arguments: &ArgMatches) -> Result<environment::Environment> {
    use environment::*;

    let mut env_builder = EnvironmentBuilder::default();

    if let Some(file_path) = arguments.value_of("environment_file") {
        // Load given environment file
        let env_file = Path::new(file_path);
        if !env_file.is_file() {
            return Err(format!("Environment file '{}' does not exist", file_path).into());
        }
        env_builder.from_file(Path::new(env_file));
    } else {
        // Try to find a environment file for the current input
        let input_file = Path::new(arguments.value_of("input_file").unwrap());
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

    if let Some(level) = arguments.value_of("optimization_level") {
        env_builder.optimization_level(match level {
            "none" => OptimizationLevel::Disabled,
            "basic" => OptimizationLevel::Basic,
            "full" => OptimizationLevel::Full,
            _ => panic!("unknown optimization level"),
        });
    }

    if let Some(check) = arguments.value_of("check") {
        env_builder.check(match check {
            "all" => Check::AllLeaks,
            "normal" => Check::OnlyNormalExecutionLeaks,
            "transient" => Check::OnlyTransientExecutionLeaks,
            _ => panic!("unknown check type"),
        });
    }

    if let Some(solver) = arguments.value_of("solver") {
        env_builder.solver(match solver {
            "z3" => Solver::Z3,
            "cvc4" => Solver::CVC4,
            "yices2" => Solver::Yices2,
            _ => panic!("unknown solver"),
        });
    }

    if arguments.is_present("debug") {
        env_builder.debug(true);
    }

    env_builder.build()
}

fn hir_transformations(env: &environment::Environment, program: &mut hir::Program) -> Result<()> {
    use hir::transformation::*;

    let transformations = {
        let mut steps: Vec<Box<dyn Transform<hir::Program>>> = Vec::new();
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
        Term::stdout().clear_line()?;
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

fn spec_bmc(arguments: &ArgMatches) -> Result<()> {
    let input_file = arguments.value_of("input_file").unwrap();

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
        loader::load_program(Path::new(input_file), arguments.value_of("function"))?;

    if let Some(path) = arguments.value_of("cfg_file") {
        hir_program
            .control_flow_graph()
            .render_to_file(Path::new(path))?;
    }

    println!("{} Transforming HIR ...", style("[4/9]").bold().dim());
    hir_transformations(&env, &mut hir_program)?;

    if let Some(path) = arguments.value_of("transient_cfg_file") {
        hir_program
            .control_flow_graph()
            .render_to_file(Path::new(path))?;
    }

    println!("{} Translating into MIR", style("[5/9]").bold().dim());
    let mir_program = mir::Program::from(&hir_program)?;

    println!("{} Translating into LIR", style("[6/9]").bold().dim());
    let mut lir_program = lir::Program::from(&mir_program)?;
    lir_program.validate()?;

    println!("{} Optimizing LIR", style("[7/9]").bold().dim());
    lir_optimize(&env, &mut lir_program)?;

    let mut solver = create_solver(&env)?;
    if let Some(path) = arguments.value_of("smt_file") {
        solver.dump_formula_to_file(Path::new(path))?
    }

    println!("{} Encoding LIR", style("[8/9]").bold().dim());
    solver.encode_program(&lir_program)?;

    if arguments.is_present("skip_solving") {
        return Ok(());
    }

    println!("{} Searching for leaks ...", style("[9/9]").bold().dim());
    match solver.check_assertions()? {
        CheckResult::AssertionsHold => {
            println!("{}", "Program is safe.".bold().green());
        }
        CheckResult::AssertionViolated => {
            println!("{}", "Leak detected!".bold().red());
            process::exit(1);
        }
    }

    Ok(())
}
