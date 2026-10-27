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
        self.val = self.root / "val"
        self.plan = self.val / "plan"
        self.plan.mkdir(parents=True)
        (self.root / "pkg/loopopt/data/global").mkdir(parents=True)
        for name in ("cmp2.py", "runval.py", "planval.py", "same.py"):
            shutil.copyfile(Path(__file__).with_name(name), self.val / name)
        self.solver = self.root / "solver"
        self.output = {
            "results": [
                {"p_all": 0.5, "p_minus1": 0.75, "trainee": {"id": 1}, "parents": [0, 1], "deck": {"card_ids": [1]}}
            ],
            # a plan whose first step (A's own career, on her side's parents at its odds) is priced at score 0: the
            # tripwire passes it
            "next": {
                "proxy_check": {"rows": []},
                "members": [
                    {"trainee": "A", "real": False, "building_careers": 0, "careers": 10.0, "parents": [0, 1], "p": 0.1}
                ],
                "options": [
                    {
                        "trainee": {"name": "A", "id": 1},
                        "role": "plan",
                        "looped": False,
                        "career": "1:0,1:x/y/0",
                        "parents": [0, 1],
                        "p_all": 0.1,
                        "score": 0.0,
                    }
                ],
                "v0": 10.0,
                "v0_plan": {"a": "A", "b": "B", "c": "C", "careers": [10.0, 0.0, 0.0]},
            },
        }
        self.write(self.plan / "jobs.json", [["global", "tiny"]])
        self.write(self.plan / "tiny.json", {})
        self.write(self.plan / "ref__tiny.out", self.output)
        self.fake_solver(self.output)

    def write(self, path, obj):
        path.write_text(json.dumps(obj))

    def fake_solver(self, output, status=0):
        self.solver.write_text(
            f"#!{sys.executable}\nimport sys\n"
            f"print({json.dumps(output)!r})\n"
            'print("solver diagnostic", file=sys.stderr)\n'
            f"sys.exit({status})\n"
        )
        self.solver.chmod(0o700)

    def run_script(self, name, *args):
        return subprocess.run([sys.executable, str(self.val / name), *map(str, args)], capture_output=True, text=True)

    def compare(self, output):
        self.write(self.val / "ref__tiny.out", self.output)
        self.write(self.val / "trial__tiny.out", output)
        return self.run_script("cmp2.py", "trial", "ref")

    def plan_run(self):
        return self.run_script("planval.py", self.solver, "trial", "-j", "1")

    def test_comparison_passes_matching_output(self):
        self.assertEqual(self.compare(self.output).returncode, 0)

    def test_comparison_fails_mismatch_parse_error_and_cap(self):
        changed = json.loads(json.dumps(self.output))
        changed["results"][0]["p_all"] = 0.25
        cap = {**self.output, "exact": {"class_max_ratio": [1.3], "class_caps": [1.25]}}
        for output in (changed, None, cap):
            with self.subTest(output=output):
                self.assertNotEqual(self.compare(output).returncode, 0)

    def test_comparison_fails_without_comparisons(self):
        self.assertNotEqual(self.run_script("cmp2.py", "trial", "ref").returncode, 0)

    def test_whole_output_ignores_timings_only(self):
        self.write(self.val / "ref__tiny.out", {**self.output, "timing_s": 1.0, "work": {"nodes": 5, "x_ms": 2}})
        same = {**self.output, "timing_s": 9.0, "work": {"nodes": 5, "x_ms": 7}}
        self.write(self.val / "trial__tiny.out", same)
        self.assertEqual(self.run_script("same.py", "trial").returncode, 0)
        self.write(self.val / "trial__tiny.out", {**same, "work": {"nodes": 6, "x_ms": 7}})
        result = self.run_script("same.py", "trial")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(".work.nodes", result.stdout)
        (self.val / "trial__tiny.out").unlink()
        self.assertNotEqual(self.run_script("same.py", "trial").returncode, 0)

    def test_whole_output_missing_key_is_not_null(self):
        self.write(self.val / "ref__tiny.out", self.output)
        self.write(self.val / "trial__tiny.out", {**self.output, "extra": None})
        result = self.run_script("same.py", "trial")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(".extra", result.stdout)

    def test_whole_output_plan_directory(self):
        self.write(self.plan / "trial__tiny.out", self.output)
        self.assertEqual(self.run_script("same.py", "trial", "--plan").returncode, 0)
        self.write(self.plan / "trial__tiny.out", {**self.output, "next": 1})
        self.assertNotEqual(self.run_script("same.py", "trial", "--plan").returncode, 0)

    def test_plan_passes_matching_output(self):
        self.assertEqual(self.plan_run().returncode, 0)

    def test_plan_fails_a_low_proxy(self):
        row = {"trainee": "A", "cand": False, "exact_val": 0.1, "same_deck": False, "score": 1.0, "proxy_score": 1.0}
        # (an efficiency 4% low; a saving 0.5 careers low)
        for eff, proxy_eff, proxy_score in (([1.0], [0.96], 1.0), ([1.0], [1.0], 0.5)):
            with self.subTest(proxy_eff=proxy_eff, proxy_score=proxy_score):
                rows = [{**row, "eff": eff, "proxy_eff": proxy_eff, "proxy_score": proxy_score}]
                output = {**self.output, "next": {**self.output["next"], "proxy_check": {"rows": rows}}}
                self.write(self.plan / "ref__tiny.out", output)
                self.fake_solver(output)
                result = self.plan_run()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("LOW", result.stdout)

    def test_plan_passes_a_small_saving_error(self):
        row = {"trainee": "A", "cand": False, "exact_val": 0.1, "same_deck": True, "score": 0.1, "proxy_score": 0.0}
        rows = [{**row, "eff": [1.0], "proxy_eff": [0.99]}]
        output = {**self.output, "next": {**self.output["next"], "proxy_check": {"rows": rows}}}
        self.write(self.plan / "ref__tiny.out", output)
        self.fake_solver(output)
        self.assertEqual(self.plan_run().returncode, 0)

    def first_step_run(self, output=None, job=None, **plan):
        """planval on the fixture with these `next` fields replaced (or a whole output), and this job"""
        output = output or {**self.output, "next": {**self.output["next"], **plan}}
        self.write(self.plan / "ref__tiny.out", output)
        self.write(self.plan / "tiny.json", job or {})
        self.fake_solver(output)
        return self.plan_run()

    def assert_first_step(self, result, fails, says):
        self.assertEqual(result.returncode != 0, fails, result.stdout)
        self.assertIn(says, result.stdout)

    def test_plan_first_step_fails_below_the_floor_only(self):
        row = self.output["next"]["options"][0]
        # (V's own first step scoring -0.5 fails; +0.5 is reported as V not its own fixed point)
        for score, fails, says in ((-0.5, True, "BELOW"), (0.5, False, "not its own fixed point")):
            with self.subTest(score=score):
                result = self.first_step_run(options=[{**row, "score": score}])
                self.assert_first_step(result, fails, says)
                self.assertIn("own full-veteran term 1.0000", result.stdout)

    def test_plan_first_step_fails_an_unpriced_ambiguous_or_masked_step(self):
        row = self.output["next"]["options"][0]
        other = {**row, "career": "1:2,3:x/y/0", "parents": [2, 3], "score": 0.5}
        for options, says in (
            ([], "no row of hers"),
            ([{**row, "score": float("nan")}], "score nan"),
            ([row, {**row, "career": "1:0,1:z/y/0"}], "ambiguous"),
            # (her other career scoring +0.5 does not stand in for the route's own at -0.5)
            ([other, {**row, "score": -0.5}], "BELOW"),
        ):
            with self.subTest(options=options):
                self.assert_first_step(self.first_step_run(options=options), True, says)

    def test_plan_first_step_without_v_needs_no_route(self):
        no_route = {"v0": None, "members": [], "v0_plan": None, "options": []}
        self.assert_first_step(self.first_step_run(**no_route), False, "not applicable")
        missing_v = {key: value for key, value in self.output["next"].items() if key != "v0"}
        for output, says in (
            ({**self.output, "next": {**self.output["next"], "v0": None}}, "null beside a route"),
            ({**self.output, "next": {**no_route, "members": None}}, "null beside a route"),
            ({**self.output, "next": missing_v}, "no `v0` field"),
            ({**self.output, "error": "no plan"}, "an error"),
        ):
            with self.subTest(says=says):
                self.assert_first_step(self.first_step_run(output=output), True, says)

    def test_plan_first_step_skips_a_given_cont_and_a_borrowing_exclusion(self):
        building = {**self.output["next"]["members"][0], "building_careers": 5.0, "builder": None}
        # (her Cont given by the job's next.fit: the solver reports no builder; without it, a missing builder fails)
        fit = {"next": {"fit": [[1, 0.1, 0.1, 0.01, 0.001]]}}
        self.assert_first_step(self.first_step_run(job=fit, members=[building]), False, "Cont given")
        self.assert_first_step(self.first_step_run(members=[building]), True, "not reported")
        # (her builder on a borrowed parent under "later" is no option; under "allow" its missing row fails)
        legacies = [{"chara": 5, "borrowed": False}, {"chara": 6, "borrowed": True}]
        borrowing = {**building, "builder": "2:0,1:x/y/0"}
        for rule, fails, says in (("later", False, "borrowing rule"), ("allow", True, "no row has its career id")):
            with self.subTest(rule=rule):
                job = {"borrowed_parent": rule, "legacies": legacies}
                self.assert_first_step(self.first_step_run(job=job, members=[borrowing]), fails, says)

    def test_plan_fails_mismatch_and_missing_reference(self):
        self.write(self.plan / "ref__tiny.out", {"different": True})
        self.assertNotEqual(self.plan_run().returncode, 0)
        (self.plan / "ref__tiny.out").unlink()
        self.assertNotEqual(self.plan_run().returncode, 0)

    def test_runners_fail_on_nonzero_solver_exit_even_with_valid_output(self):
        self.fake_solver(self.output, status=3)
        (self.val / "jobs").mkdir()
        self.write(self.val / "jobs.json", [["global", "tiny"]])
        self.write(self.val / "jobs/tiny.json", {"mode": "career"})
        for name in ("runval.py", "planval.py"):
            with self.subTest(script=name):
                result = self.run_script(name, self.solver, "trial", "-j", "1")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("solver exited 3", result.stderr)
                self.assertIn("solver diagnostic", result.stderr)


if __name__ == "__main__":
    unittest.main()
