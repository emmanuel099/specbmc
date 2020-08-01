#!/usr/bin/env python3

from subprocess import Popen, PIPE, TimeoutExpired
import shlex
import glob
import yaml
import time

SOLVER = 'yices2'
DEFAULT_ARGS = '--skip-cex'
TIMEOUT = 60  # seconds

# specbmc exit codes
EXIT_CODE_SAFE = 0
EXIT_CODE_UNSAFE = 2


def str_without_suffix(s: str, suffix: str) -> str:
    if suffix and s.endswith(suffix):
        return s[:-len(suffix)]
    else:
        return s


def get_expected_value_from_environment(env_file: str):
    with open(env_file, 'r') as f:
        env = yaml.safe_load(f)
        expected = env['test']['expect']
        if not expected in ['safe', 'unsafe']:
            raise Exception('expected value must be safe or unsafe')
        return expected


def run_test(test_file: str, env_file: str):
    print(f'test {test_file} with environment {env_file} ... ',
          end='', flush=True)

    try:
        expected = get_expected_value_from_environment(env_file)
    except:
        print('error, failed get expected value from environment')
        return False

    try:
        args = ['specbmc', test, '--env', env, '--solver', SOLVER]
        args += shlex.split(DEFAULT_ARGS)
        start_time = time.time()
        proc = Popen(args, stdout=PIPE, stderr=PIPE)
        (output, err) = proc.communicate(timeout=TIMEOUT)
        exit_code = proc.wait(timeout=TIMEOUT)
        end_time = time.time()
    except TimeoutExpired:
        proc.kill()
        print('timeout')
        return False

    if not exit_code in [EXIT_CODE_SAFE, EXIT_CODE_UNSAFE]:
        print('error, output was:')
        print(output.decode())
        print(err.decode())
        return False

    actual = 'safe' if exit_code == EXIT_CODE_SAFE else 'unsafe'
    elapsed = '{:0.2f}s'.format(end_time - start_time)

    if expected != actual:
        print(f'failed, expected {expected} but was {actual}, took {elapsed}')
        return False

    print(f'ok, took {elapsed}')
    return True


def find_tests():
    return glob.glob('*.muasm', recursive=False)


def find_environments_for_test(test: str):
    env_prefix = str_without_suffix(test, '.muasm')
    return glob.glob(f'{env_prefix}.*yaml', recursive=False)


print(f'SOLVER={SOLVER}')
print(f'DEFAULT_ARGS={DEFAULT_ARGS}')
print(f'TIMEOUT={TIMEOUT}s')

some_test_failed = False

tests = find_tests()
print(f'found {len(tests)} tests')
print()

for test in tests:
    for env in find_environments_for_test(test):
        if not run_test(test, env):
            some_test_failed = True

print()

if some_test_failed:
    print("Some tests failed!")
    exit(1)

print("All tests passed.")
exit(0)
