#!/usr/bin/env python3

from subprocess import Popen, PIPE, TimeoutExpired
import shlex
import yaml
import time
import os
from pathlib import Path

SOLVER = os.getenv('SOLVER', '')
DEFAULT_ARGS = os.getenv('DEFAULT_ARGS', '--skip-cex')
SPECBMC_BIN = os.getenv('SPECBMC_BIN', 'target/debug/specbmc')
TIMEOUT = os.getenv('TIMEOUT', 60)  # seconds

# specbmc exit codes
EXIT_CODE_SAFE = 0
EXIT_CODE_UNSAFE = 2


def get_expected_value_from_environment(env_file: str):
    with open(env_file, 'r') as f:
        env = yaml.safe_load(f)
        expected = env['test']['expect']
        if not expected in ['safe', 'unsafe']:
            raise Exception('expected value must be safe or unsafe')
        return expected


def run_test(test_file, env_file):
    try:
        expected = get_expected_value_from_environment(env_file)
    except:
        print('error, failed get expected value from environment')
        return False

    try:
        args = [SPECBMC_BIN, test_file, '--env', env_file]
        if SOLVER:
            args += ['--solver', SOLVER]
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
    return Path('test').rglob('*.muasm')


def find_environments_for_test(test_file):
    env_prefix = test_file.stem
    return test_file.parent.glob(f'{env_prefix}.*yaml')


print(f'SOLVER={SOLVER}')
print(f'DEFAULT_ARGS={DEFAULT_ARGS}')
print(f'SPECBMC_BIN={SPECBMC_BIN}')
print(f'TIMEOUT={TIMEOUT}s')

print()

failed_tests = []

for test_file in find_tests():
    for env_file in find_environments_for_test(test_file):
        test_name = f'{test_file} with environment {env_file}'
        print(f'test {test_name} ... ', end='', flush=True)
        if not run_test(test_file, env_file):
            failed_tests.append(test_name)

print()

if failed_tests:
    print(f'{len(failed_tests)} tests failed!')
    print('\nFailed tests:')
    for failed_test in failed_tests:
        print(f'  - {failed_test}')
    exit(1)

print('All tests passed.')
exit(0)
