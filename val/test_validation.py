"""Check validation failures without running the solver: uv run val/test_validation.py."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


class ValidationExitTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.val = self.root / 'val'
        self.plan = self.val / 'plan'
        self.plan.mkdir(parents=True)
        (self.root / 'pkg/loopopt/data/global').mkdir(parents=True)
        for name in ('cmp2.py', 'runval.py', 'planval.py', 'same.py'):
            shutil.copyfile(Path(__file__).with_name(name), self.val / name)
        self.solver = self.root / 'solver'
        self.output = {'results': [{'p_all': 0.5, 'p_minus1': 0.75,
                                   'trainee': {'id': 1}, 'parents': [0, 1],
                                   'deck': {'card_ids': [1]}}]}
        self.write(self.plan / 'jobs.json', [['global', 'tiny']])
        self.write(self.plan / 'tiny.json', {})
        self.write(self.plan / 'ref__tiny.out', self.output)
        self.fake_solver(self.output)

    def write(self, path, obj):
        path.write_text(json.dumps(obj))

    def fake_solver(self, output, status=0):
        self.solver.write_text(f'#!{sys.executable}\nimport sys\n'
                               f'print({json.dumps(output)!r})\n'
                               'print("solver diagnostic", file=sys.stderr)\n'
                               f'sys.exit({status})\n')
        self.solver.chmod(0o700)

    def run_script(self, name, *args):
        return subprocess.run([sys.executable, str(self.val / name), *map(str, args)],
                              capture_output=True, text=True)

    def compare(self, output):
        self.write(self.val / 'ref__tiny.out', self.output)
        self.write(self.val / 'trial__tiny.out', output)
        return self.run_script('cmp2.py', 'trial', 'ref')

    def plan_run(self):
        return self.run_script('planval.py', self.solver, 'trial', '-j', '1')

    def test_comparison_passes_matching_output(self):
        self.assertEqual(self.compare(self.output).returncode, 0)

    def test_comparison_fails_mismatch_parse_error_and_cap(self):
        changed = json.loads(json.dumps(self.output))
        changed['results'][0]['p_all'] = 0.25
        cap = {**self.output, 'exact': {'class_max_ratio': [1.3], 'class_caps': [1.25]}}
        for output in (changed, None, cap):
            with self.subTest(output=output):
                self.assertNotEqual(self.compare(output).returncode, 0)

    def test_comparison_fails_without_comparisons(self):
        self.assertNotEqual(self.run_script('cmp2.py', 'trial', 'ref').returncode, 0)

    def test_whole_output_ignores_timings_only(self):
        self.write(self.val / 'ref__tiny.out', {**self.output, 'timing_s': 1.0, 'work': {'nodes': 5, 'x_ms': 2}})
        same = {**self.output, 'timing_s': 9.0, 'work': {'nodes': 5, 'x_ms': 7}}
        self.write(self.val / 'trial__tiny.out', same)
        self.assertEqual(self.run_script('same.py', 'trial').returncode, 0)
        self.write(self.val / 'trial__tiny.out', {**same, 'work': {'nodes': 6, 'x_ms': 7}})
        result = self.run_script('same.py', 'trial')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('.work.nodes', result.stdout)
        (self.val / 'trial__tiny.out').unlink()
        self.assertNotEqual(self.run_script('same.py', 'trial').returncode, 0)   # nothing compared

    def test_plan_passes_matching_output(self):
        self.assertEqual(self.plan_run().returncode, 0)

    def test_plan_fails_mismatch_and_missing_reference(self):
        self.write(self.plan / 'ref__tiny.out', {'different': True})
        self.assertNotEqual(self.plan_run().returncode, 0)
        (self.plan / 'ref__tiny.out').unlink()
        self.assertNotEqual(self.plan_run().returncode, 0)

    def test_runners_fail_on_nonzero_solver_exit_even_with_valid_output(self):
        self.fake_solver(self.output, status=3)
        (self.val / 'jobs').mkdir()
        self.write(self.val / 'jobs.json', [['global', 'tiny']])
        self.write(self.val / 'jobs/tiny.json', {'mode': 'career'})
        for name in ('runval.py', 'planval.py'):
            with self.subTest(script=name):
                result = self.run_script(name, self.solver, 'trial', '-j', '1')
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('solver exited 3', result.stderr)
                self.assertIn('solver diagnostic', result.stderr)


if __name__ == '__main__':
    unittest.main()
