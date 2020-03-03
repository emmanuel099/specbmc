#[macro_use]
extern crate clap;
use clap::{Arg, ArgMatches};
use colored::*;
use console::style;
use console::Term;
use rsmt2::Solver;
use specbmc::environment;
use specbmc::error::Result;
use specbmc::loader;
use specbmc::translator;
use specbmc::util::{RenderGraph, Transform, Validate};
use specbmc::{hir, lir};
use std::fs::File;
use std::path::Path;
use std::process;

fn main() {
    let arguments = app_from_crate!()
        .arg(
            Arg::with_name("environment_file")
                .short("e")
                .long("env")
                .value_name("FILE")
                .help("Sets the environment file to use")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("optimization_level")
                .short("o")
                .long("opt")
                .value_name("LEVEL")
                .possible_values(&["none", "basic", "full"])
                .help("Sets the optimization level (overwrites env settings)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("check")
                .short("c")
                .long("check")
                .value_name("TYPE")
                .possible_values(&["all", "normal", "transient"])
                .help("Sets the leak check type (overwrites env settings)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("solver")
                .long("solver")
                .value_name("SOLVER")
                .possible_values(&["z3", "cvc4", "yices2"])
                .help("Sets the solver to use (overwrites env settings)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("function")
                .long("func")
                .value_name("NAME|ID")
                .help("Sets the name/id of the function to be checked")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("debug")
                .short("d")
                .long("debug")
                .help("Enables the debug mode"),
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
                .help("Prints the CFG into the file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("transient_cfg_file")
                .long("trans-cfg")
                .value_name("FILE")
                .help("Prints the CFG (with transient behavior) into the file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("smt_file")
                .long("smt")
                .value_name("FILE")
                .help("Prints the SMT-2 formula into the file")
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

    if let Some(file) = arguments.value_of("environment_file") {
        env_builder.from_file(Path::new(file));
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

    println!("{} Translating HIR to MIR", style("[5/9]").bold().dim());
    let mir_program = translator::hir_to_mir::translate_program(&hir_program)?;

    println!("{} Translating MIR to LIR", style("[6/9]").bold().dim());
    let mut lir_program = translator::mir_to_lir::translate_program(&mir_program)?;
    lir_program.validate()?;

    println!("{} Optimizing LIR ...", style("[7/9]").bold().dim());
    lir_optimize(&env, &mut lir_program)?;

    let parser = ();
    let mut solver = match env.solver() {
        environment::Solver::Z3 => Solver::default_z3(parser)?,
        environment::Solver::CVC4 => Solver::default_cvc4(parser)?,
        environment::Solver::Yices2 => Solver::default_yices_2(parser)?,
    };

    if let Some(path) = arguments.value_of("smt_file") {
        let file = File::create(Path::new(path))?;
        solver.tee(file)?;
    }

    println!(
        "{} Encoding LIR as SMT formula",
        style("[8/9]").bold().dim()
    );
    translator::lir_to_smt::encode_program(&mut solver, &lir_program)?;

    if arguments.is_present("skip_solving") {
        solver.print_check_sat()?;
    } else {
        println!("{} Solving SMT formula ...", style("[9/9]").bold().dim());
        let is_sat = solver.check_sat()?;
        if is_sat {
            println!("{}", "Leak detected!".bold().red());
            process::exit(1);
        } else {
            println!("{}", "Program is safe.".bold().green());
        }
    }

    Ok(())
}
