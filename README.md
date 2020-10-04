# specbmc

[![Actions](https://github.com/emmanuel099/specbmc/workflows/CI/badge.svg?branch=master)](https://github.com/emmanuel099/specbmc/actions)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

Bounded (software) model checker for speculative non-interference.

`specbmc` automatically detects Spectre-PHT and Spectre-STL vulnerabilites in binary programs.

![specbmc](doc/cli.gif)

Please note that `specbmc` has been implemented as part of my Master's Thesis and is therefore not considered "production ready".

This work has been inspired by [Spectector](https://spectector.github.io/).

## Usage

Input files are currently limited to µASM files and ELF binaries.

### Command Line

```
specbmc 0.1.0
Bounded model checker for speculative non-interference.

USAGE:
specbmc [FLAGS] [OPTIONS] <FILE>

FLAGS:
    -d, --debug            Enables debug mode
    -h, --help             Prints help information
    -a, --assembly-info    Prints assembly info and exits
        --show-env         Prints the environment to console
        --skip-cex         Skips generating counterexample
        --skip-solving     Skips solving SMT formula
    -V, --version          Prints version information

OPTIONS:
        --call-graph <FILE>          Prints call graph into file (DOT)
        --cex <FILE>                 Prints counterexample into file (DOT) [default: cex.dot]
        --cfg <FILE>                 Prints control-flow graph into file (DOT)
    -c, --check <TYPE>               Sets leak check type [possible values: all, normal, transient]
    -e, --env <FILE>                 Sets environment file to use (arguments overwrite it)
        --lir <FILE>                 Prints LIR program into file (plain text)
        --loop-tree <FILE>           Prints loop tree into file (DOT)
        --mir <FILE>                 Prints MIR program into file (DOT)
        --model <MODEL>              Sets analysis model type [possible values: components, pc]
        --observe <OBSERVE>          Sets observation type [possible values: sequential, parallel, full, trace]
    -o, --opt <LEVEL>                Sets optimization level [possible values: none, basic, full]
    -p, --predictor <STRATEGY>       Sets predictor strategy [possible values: invert, choose]
        --entry <NAME|ADDRESS>       Sets name/address of the program entry function
    -r, --recursion <LIMIT>          Inlines recursive functions at most LIMIT times
        --smt <FILE>                 Prints SMT-2 formula into file (plain text)
        --solver <SOLVER>            Sets solver [possible values: z3, cvc4, yices2]
    -s, --spec-win <WINDOW>          Sets maximum length of the speculation window
        --trans-cfg <FILE>           Prints CFG (with transient behavior) into file (DOT)
    -k, --unwind <k>                 Unwinds loops k times
        --unwinding-guard <GUARD>    Sets unwinding guard [possible values: assumption, assertion]

ARGS:
    <FILE>    Input file to be checked
```

#### Examples:

* Simple check: `specbmc --solver yices2 -k 10 -r 5 --skip-cex example.muasm`
* With environment: `specbmc -e example_env.yaml example.muasm`
* Print CFG and call-graph: `specbmc --solver cvc4 -k 10 -r 5 --call-graph cg.dot --cfg cfg.dot --entry "main" example.o`
* Print SMT formula: `specbmc -k 10 -r 5 --smt formula.txt example.muasm`
* List functions and entry point: `specbmc --assembly-info example.o`

### Environment File

Almost everything of `specbmc` can be configured via the environment file.
An environment file can be loaded via the `--env` command-line option.
Please note that command-line arguments have precedence over environment settings,
meaning that if the environment contains e.g. `optimization: full` but the option `-o none` is given, no optimization will be done.
It is required that the environment file is a valid YAML file.

#### Available options (for missing options the specified default value will be used):

```yaml
# LIR optimization level: none, basic, full [default: full]
#   - none: no optimizations
#   - basic: copy propagation
#   - full: constant folding & propagation, expression simplification and copy propagation
optimization: full

# SMT solver: z3, cvc4, yices2 [default: yices2]
solver: yices2

# Analysis
analysis:
  # Search for Spectre-PHT? false, true [default: true]
  spectre_pht: true
  # Search for Spectre-STL? false, true [default: false]
  spectre_stl: false
  # Type of leak check: only_transient_leaks, only_normal_leaks, all_leaks
  #                     [default: only_transient_leaks]
  #   - only_transient_leaks: Only find leaks which are there because of transient execution
  #   - only_normal_leaks: Find normal leaks (no transient execution)
  #   - all_leaks: Search for both types of leaks (transient + normal)
  check: only_transient_leaks
  # Branch prediction strategy: choose_path, invert_condition [default: choose_path]
  #   - choose_path: predict taken/not-taken
  #   - invert_condition: mis-predict (take the opposite)
  predictor_strategy: choose_path
  # The default number of loop iterations to unwind: n >= 0 [default: 0]
  unwind: 0
  # The number of loop iterations to unwind for specific loops (key is loop id, value is unwinding bound >= 0)
  # If no specific loop bound is given, the default unwinding bound is used instead.
  unwind_loop:
    ...
  # Add either unwinding assumptions or assertions: assumption, assertion [default: assumption]
  unwinding_guard: assumption
  # Recursion limit for recursive function-inlining: # n >= 0 [default: 0]
  recursion_limit: 0
  # Start with empty (flushed) cache? false, true [default: false]
  # Note: This option is currently only available when using the CVC4 solver.
  start_with_empty_cache: false
  # Type of observation: sequential, parallel, full [default: parallel]
  #   - sequential: Observe only at the end of the program.
  #                 Transient execution can resolve at any time.
  #   - parallel:   Observe each instruction and control-flow join.
  #                 Transient execution resolves only if speculation window is exceeded.
  #                 Much cheaper than `full` but may miss some special types of control-flow leaks,
  #                 see `test/window_branch_leak_size_three.muasm`.
  #   - full:       Same as parallel but transient execution can resolve at any time.
  #   - trace:      Same as parallel but full trace instead of individual observations.
  observe: parallel
  # Type of analysis model: components, pc [default: components]
  #   - components: Observe microarchitectual components like cache, branch-target buffer, ...
  #   - pc:         Observe program counter and memory loads (cheaper than components model)
  model: components
  # The program entry point: string [default: entry point from binary]
  program_entry: "main"
  # List of function names which should not be inlined
  inline_ignore: []

# Architecture
architecture:
  # Is cache available to attacker? false, true [default: true]
  cache: true
  # Is branch target buffer available to attacker? false, true [default: true]
  btb: true
  # Is pattern history table available to attacker? false, true [default: true]
  pht: true
  # The length of the speculation window: n >= 0 [default: 100]
  speculation_window: 100

# Security policy
policy:
  registers:
    # The default security policy of all registers: low, high [default: low]
    default: low
    # List of high-security registers [default: empty] (only makes sense when default is low)
    high: []
    # List of low-security registers [default: empty] (only makes sense when default is high)
    low: []
  memory: # Memory locations defined by sections with start and end address (end is exclusive)
    # The default security policy of all memory locations: low, high [default: high]
    default: high
    # List of high-security memory locations [default: empty] (only makes sense when default is low)
    high: []
    # List of low-security memory locations [default: empty] (only makes sense when default is high)
    low: []

# Initial Setup
setup:
  # Prepare stack (0xffff_0000_0000 < rsp <= rbp) and return address
  init_stack: false
  # Initial register content (key is register name, value is register content)
  registers:
    ...
  # Initial flag register content (key is flag name, value is boolean)
  flags:
    ...
  # Initial memory content (key is address, value is sequence of bytes)
  memory:
    ...

# Debug mode: false, true [default: false]
debug: false
```

#### Example Environment:

```yaml
optimization: full
solver: yices2
analysis:
  spectre_pht: true
  spectre_stl: false
  check: only_transient_leaks
  unwind: 10
  unwind_loop:
    0x42: 30
    0x100: 5
  recursion_limit: 5
  program_entry: "main"
architecture:
  cache: true
  btb: true
  pht: true
  speculation_window: 100
policy:
  registers:
    default: low
    high: ["secret"]
  memory:
    default: high
    low:
      - # section A
        start: 0x200000
        end: 0x201000
      - # section B
        start: 0x300000
        end: 0x301000
setup:
  registers:
    rdi: 0x10
    rsi: 0x11
  flags:
    DF: false
  memory:
    0x10: [0x0a, 0x0b]
    0x12: [0x0c]
```

#### Environment Auto-loading:

If the `--env` command-line option isn't set, `specbmc` will automatically search for a matching environment file in the directory where the input file `<FILE>` is located.
If no matching environment file could be found, the default values will be used instead.

By convention `specbmc` assumes that for an input file `{name}{extension}` an environment file `{name}.yaml` exists. For example, if the input file is `example.o` then `specbmc` will search for an environment file `example.yaml` in the same directory.
