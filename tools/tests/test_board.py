"""Offline tests of the rules in `tools/board`.

They build a `Snapshot` by hand and check what the board decides from it, so
nothing here contacts GitHub. Run them with `mise run test:tools`.
"""

import datetime
import importlib.machinery
import importlib.util
import pathlib
import re
import subprocess
import sys
import unittest

sys.dont_write_bytecode = True

_PATH = pathlib.Path(__file__).resolve().parent.parent / "board"
_LOADER = importlib.machinery.SourceFileLoader("board", str(_PATH))
_SPEC = importlib.util.spec_from_loader("board", _LOADER)
board = importlib.util.module_from_spec(_SPEC)
_LOADER.exec_module(board)

REPO = board.REPO
PARENT = board.PARENT


def issue(number, *, blockers=(), closed=False, children=False, parent=PARENT, depth=0,
		assignees=("rye",), priority="High"):
	return board.Issue(
		number=number, title=f"Issue {number}", url=f"https://github.com/{REPO}/issues/{number}",
		closed=closed, depth=depth, parent=parent, assignees=list(assignees), has_children=children,
		blockers=[b if isinstance(b, board.Blocker) else board.Blocker(b, REPO, False) for b in blockers],
		priority=priority,
	)


def closed_blocker(number):
	return board.Blocker(number, REPO, True)


def item(number, *, status="Backlog", size="S", estimate=1.0, iteration="Iteration 1"):
	return board.Item(id=f"ITEM_{number}", kind="Issue", number=number, repository=REPO, title=None,
		status=status, size=size, estimate=estimate, iteration=iteration)


def pull(number, issue_number, *, base=board.BASE, draft=False, name="x"):
	return board.Pull(number=number, title="t", head=f"rearch/{issue_number}-{name}", base=base,
		draft=draft, review="none", checks="passing")


def snapshot(issues, items=None, pulls=(), merged=(), foreign=()):
	if items is None:
		items = [item(i.number) for i in issues]
	return board.Snapshot(issues=list(issues), foreign=list(foreign), items=list(items),
		pulls=list(pulls), merged_issues=set(merged))


def row(snap, number):
	return next(r for r in board.rows(snap) if r.number == number)


def plan(snap, branches=()):
	return board.plan(board.rows(snap), snap, set(branches))


class Readiness(unittest.TestCase):
	def test_no_blockers_starts_from_base(self):
		r = row(snapshot([issue(1)]), 1)
		self.assertTrue(r.ready)
		self.assertEqual(r.base, board.BASE)

	def test_closed_blocker_does_not_block(self):
		self.assertTrue(row(snapshot([issue(1, blockers=[closed_blocker(2)])]), 1).ready)

	def test_blocker_with_merged_pull_request_does_not_block(self):
		self.assertTrue(row(snapshot([issue(1, blockers=[2]), issue(2)], merged=[2]), 1).ready)

	def test_blocker_without_pull_request_blocks(self):
		r = row(snapshot([issue(1, blockers=[2]), issue(2)]), 1)
		self.assertFalse(r.ready)
		self.assertEqual(r.waiting_reason, "#2 has no pull request")

	def test_blocker_with_draft_pull_request_blocks(self):
		r = row(snapshot([issue(1, blockers=[2]), issue(2)], pulls=[pull(10, 2, draft=True)]), 1)
		self.assertFalse(r.ready)
		self.assertEqual(r.waiting_reason, "#2 is a draft")

	def test_blocker_with_several_pull_requests_blocks(self):
		snap = snapshot([issue(1, blockers=[2]), issue(2)], pulls=[pull(10, 2, name="a"), pull(11, 2, name="b")])
		self.assertEqual(row(snap, 1).waiting_reason, "#2 has several pull requests")

	def test_starts_from_blockers_branch(self):
		r = row(snapshot([issue(1, blockers=[2]), issue(2)], pulls=[pull(10, 2)]), 1)
		self.assertTrue(r.ready)
		self.assertEqual(r.base, "rearch/2-x")

	def test_starts_from_highest_branch_of_one_chain(self):
		pulls = [pull(10, 2), pull(11, 3, base="rearch/2-x")]
		r = row(snapshot([issue(1, blockers=[2, 3]), issue(2), issue(3)], pulls=pulls), 1)
		self.assertEqual(r.base, "rearch/3-x")

	def test_blockers_on_separate_branches_block(self):
		pulls = [pull(10, 2), pull(11, 3)]
		r = row(snapshot([issue(1, blockers=[2, 3]), issue(2), issue(3)], pulls=pulls), 1)
		self.assertFalse(r.ready)
		self.assertEqual(r.waiting_reason, "blockers are on separate branches")

	def test_broken_chain_blocks(self):
		pulls = [pull(11, 3, base="rearch/2-x")]
		r = row(snapshot([issue(1, blockers=[3]), issue(3)], pulls=pulls), 1)
		self.assertEqual(r.waiting_reason, "a blocker's pull request does not reach rearchitecture")

	def test_open_blocker_in_another_repository_blocks(self):
		blocker = board.Blocker(5, "frog-pond/ccc-server", False)
		r = row(snapshot([issue(1, blockers=[blocker])]), 1)
		self.assertFalse(r.ready)
		self.assertEqual(r.waiting_reason, "frog-pond/ccc-server#5 is open")

	def test_parent_is_never_ready(self):
		r = row(snapshot([issue(1, children=True), issue(2, parent=1, depth=1)]), 1)
		self.assertFalse(r.ready)
		self.assertEqual(r.waiting_reason, "has sub-issues; work those")

	def test_issue_with_own_pull_request_is_not_ready(self):
		self.assertFalse(row(snapshot([issue(1)], pulls=[pull(10, 1)]), 1).ready)

	def test_started_issue_is_not_ready(self):
		self.assertFalse(row(snapshot([issue(1)], items=[item(1, status="In progress")]), 1).ready)


class Moves(unittest.TestCase):
	def kinds(self, moves):
		return [(type(m).__name__, getattr(m, "status", None)) for m in moves]

	def test_closed_issue_is_done(self):
		moves, _ = plan(snapshot([issue(1, closed=True)]))
		self.assertEqual(self.kinds(moves), [("SetStatus", "Done")])

	def test_ready_issue_moves_to_ready(self):
		moves, _ = plan(snapshot([issue(1)]))
		self.assertEqual(self.kinds(moves), [("SetStatus", "Ready")])

	def test_blocked_again_goes_back_to_backlog(self):
		moves, _ = plan(snapshot([issue(1, blockers=[2]), issue(2)], items=[item(1, status="Ready"), item(2)]))
		self.assertIn(("SetStatus", "Backlog"), self.kinds(moves))

	def test_draft_pull_request_is_in_progress_and_ready_one_in_review(self):
		moves, _ = plan(snapshot([issue(1), issue(2)], pulls=[pull(10, 1, draft=True), pull(11, 2)]))
		self.assertEqual(self.kinds(moves), [("SetStatus", "In progress"), ("SetStatus", "In review")])

	def test_parent_follows_its_sub_issues(self):
		issues = [issue(1, children=True), issue(2, parent=1, depth=1)]
		moves, _ = plan(snapshot(issues, items=[item(1), item(2, status="In progress")]), branches=[2])
		self.assertEqual(self.kinds(moves), [("SetStatus", "In progress")])

	def test_missing_issue_is_added_then_put_in_iteration(self):
		moves, _ = plan(snapshot([issue(1)], items=[]))
		self.assertEqual(self.kinds(moves), [("AddToBoard", "Ready"), ("SetIteration", None)])

	def test_unassigned_open_issue_is_assigned(self):
		moves, _ = plan(snapshot([issue(1, assignees=())], items=[item(1, status="Ready")]))
		self.assertEqual(self.kinds(moves), [("Assign", None)])

	def test_iteration_is_set_after_add_on_the_new_item(self):
		"""The fault Copilot found in #447: the iteration move ran with no item id."""
		moves, _ = plan(snapshot([issue(1)], items=[]))
		project = FakeProject()
		for move in moves:
			move.apply(project)
		self.assertEqual(project.calls, [("add", moves[0].row.url), ("status", "NEW", "Ready"), ("iteration", "NEW")])


class Notes(unittest.TestCase):
	def notes(self, snap, branches=()):
		return plan(snap, branches)[1]

	def test_merged_pull_request_on_open_issue(self):
		self.assertIn("#1: its pull request merged; check its \"Done when\" and close it",
			self.notes(snapshot([issue(1)], merged=[1])))

	def test_every_sub_issue_closed(self):
		snap = snapshot([issue(1, children=True), issue(2, parent=1, depth=1, closed=True)])
		self.assertIn("#1: every sub-issue is closed; check its \"Done when\" and close it", self.notes(snap))

	def test_open_but_done(self):
		self.assertIn("#1: open but marked Done; reopened, or closed by mistake?",
			self.notes(snapshot([issue(1)], items=[item(1, status="Done")])))

	def test_in_progress_without_branch_is_left_alone(self):
		moves, notes = plan(snapshot([issue(1)], items=[item(1, status="In progress")]))
		self.assertEqual(moves, [])
		self.assertIn("#1: In progress, but has no branch and no pull request; is anyone working it?", notes)

	def test_in_review_with_branch_but_no_pull_request(self):
		snap = snapshot([issue(1)], items=[item(1, status="In review")])
		self.assertIn("#1: In review, but its branch has no open pull request", self.notes(snap, branches=[1]))

	def test_missing_size_priority_and_estimate(self):
		snap = snapshot([issue(1, priority=None)], items=[item(1, size=None, estimate=None, status="Ready")])
		self.assertIn("#1: no size, no priority, no estimate", self.notes(snap))

	def test_parent_is_not_asked_for_an_estimate(self):
		snap = snapshot([issue(1, children=True, priority=None), issue(2, parent=1, depth=1)],
			items=[item(1, estimate=None), item(2)])
		self.assertFalse(any(n.startswith("#1: no") for n in self.notes(snap)))

	def test_stray_foreign_and_draft_items(self):
		draft = board.Item(id="D", kind="DraftIssue", number=None, repository=None, title="Idea",
			status=None, size=None, estimate=None, iteration=None)
		pr = board.Item(id="P", kind="PullRequest", number=7, repository=REPO, title=None,
			status=None, size=None, estimate=None, iteration=None)
		snap = snapshot([issue(1)], items=[item(1), item(99), draft, pr],
			foreign=[board.ForeignIssue(3, "other/repo", PARENT, False)])
		notes = self.notes(snap)
		self.assertIn(f"#99: on the board but not under #{PARENT}", notes)
		self.assertIn("other/repo#3 (under #424): a sub-issue in another repository; tools/board does not follow it", notes)
		self.assertIn("draft \"Idea\": waiting for the maintainer to review it and file it", notes)
		self.assertIn(f"pull request {REPO}#7 is on the board; tools/board ignores it", notes)

	def test_open_foreign_sub_issue_keeps_parent_open(self):
		"""Codex, round 2: the parent was reported ready to close."""
		snap = snapshot([issue(1, children=True), issue(2, parent=1, depth=1, closed=True)],
			foreign=[board.ForeignIssue(3, "other/repo", 1, False)])
		self.assertFalse(any("every sub-issue is closed" in n for n in self.notes(snap)))

	def test_closed_foreign_sub_issue_counts(self):
		snap = snapshot([issue(1, children=True), issue(2, parent=1, depth=1, closed=True)],
			foreign=[board.ForeignIssue(3, "other/repo", 1, True)])
		self.assertIn("#1: every sub-issue is closed; check its \"Done when\" and close it", self.notes(snap))


class Output(unittest.TestCase):
	def test_estimate_line_names_issues_without_one(self):
		snap = snapshot([issue(1), issue(2), issue(3, closed=True), issue(4, children=True)],
			items=[item(1, estimate=2.5), item(2, estimate=None), item(3, estimate=4), item(4, estimate=None)])
		self.assertEqual(board.estimate_line(board.rows(snap)),
			"Estimated hours left: 2.5, over 1 of the 2 open issues without sub-issues; no estimate on #2")

	def test_table_shows_missing_values_as_dashes(self):
		snap = snapshot([issue(1, priority=None)], items=[])
		line = board.table(board.rows(snap))[3]
		self.assertTrue(line.startswith("→ #1     -            -       -        -  -"), line)

	def test_foreign_blocker_shows_its_repository(self):
		"""Claude, round 2: it showed as this repository's #5."""
		snap = snapshot([issue(1, blockers=[board.Blocker(5, "frog-pond/ccc-server", False), 2]), issue(2)])
		r = row(snap, 1)
		self.assertEqual((r.open_blockers, r.foreign_blockers), ([2], ["frog-pond/ccc-server#5"]))
		self.assertIn("#2,frog-pond/ccc-server#5", board.table([r])[3])

	def test_json_keeps_dashes_for_missing_status_and_size(self):
		"""Codex, round 2: --json had changed them to null."""
		data = board.json_row(row(snapshot([issue(1, priority=None)], items=[]), 1))
		self.assertEqual((data["status"], data["size"], data["priority"], data["estimate"]), ("-", "-", None, None))

	def test_current_iteration(self):
		iterations = [{"id": "A", "startDate": "2026-11-22", "duration": 14}]
		self.assertEqual(board.current_iteration(iterations, datetime.date(2026, 12, 5))["id"], "A")
		self.assertIsNone(board.current_iteration(iterations, datetime.date(2026, 12, 6)))


class Reading(unittest.TestCase):
	def test_priority_is_read_from_issue_fields(self):
		node = {
			"number": 1, "title": "t", "url": "u", "state": "OPEN", "repository": {"nameWithOwner": REPO},
			"assignees": {"nodes": [{"login": "rye"}]}, "subIssuesSummary": {"total": 0},
			"blockedBy": {"nodes": [
				{"number": 9, "state": "OPEN", "repository": {"nameWithOwner": REPO}},
				{"number": 3, "state": "CLOSED", "repository": {"nameWithOwner": REPO}},
			]},
			"issueFieldValues": {"nodes": [{}, {"name": "Low", "field": {"name": "Effort"}},
				{"name": "Urgent", "field": {"name": "Priority"}}]},
		}
		parsed = board.parse_issue(node, 0, PARENT)
		self.assertEqual(parsed.priority, "Urgent")
		self.assertEqual([b.number for b in parsed.blockers], [3, 9])
		self.assertEqual([b.closed for b in parsed.blockers], [True, False])

	def test_null_issue_fields_are_read_as_none(self):
		"""Codex, round 2: a null connection or entry raised AttributeError."""
		base = {"number": 1, "title": "t", "url": "u", "state": "OPEN", "repository": {"nameWithOwner": REPO},
			"assignees": None, "subIssuesSummary": None, "blockedBy": {"nodes": [None]}}
		for fields in (None, {"nodes": None}, {"nodes": [None]}):
			parsed = board.parse_issue(dict(base, issueFieldValues=fields), 0, PARENT)
			self.assertEqual((parsed.priority, parsed.assignees, parsed.blockers), (None, [], []))

	def test_a_wide_level_is_read_in_batches(self):
		"""Codex and Claude, round 2: one query for a whole level could pass GitHub's node limit."""
		parents = list(range(100, 100 + board.TREE_BATCH + 3))

		def node(n, kids):
			return {"number": n, "title": "t", "url": "u", "state": "OPEN", "repository": {"nameWithOwner": REPO},
				"assignees": {"nodes": []}, "subIssuesSummary": {"total": kids}, "blockedBy": {"nodes": []},
				"issueFieldValues": {"nodes": []}}

		queries = []

		def fake(query, **_):
			asked = [int(n) for n in re.findall(r"\bp(\d+):", query)]
			queries.append(asked)
			data = {}
			for n in asked:
				kids = [node(c, 1) for c in parents] if n == PARENT else [node(n * 10, 0)]
				data[f"p{n}"] = {"subIssues": {"nodes": kids}}
			return {"repository": data}

		real = board.graphql
		board.graphql = fake
		try:
			issues, foreign = board.fetch_tree()
		finally:
			board.graphql = real
		self.assertEqual([len(q) for q in queries], [1, board.TREE_BATCH, 3])
		self.assertEqual(len(issues), 2 * len(parents))
		self.assertEqual((issues[0].number, issues[1].number, issues[1].parent), (100, 1000, 100))
		self.assertEqual(foreign, [])

	def test_a_second_page_stops_the_tool(self):
		with self.assertRaises(SystemExit):
			board.whole({"pageInfo": {"hasNextPage": True}, "nodes": []}, "#1's sub-issues")

	def test_board_item_fields(self):
		node = {"id": "I", "content": {"__typename": "Issue", "number": 5, "repository": {"nameWithOwner": REPO}},
			"fieldValues": {"nodes": [{}, {"name": "Ready", "field": {"name": "Status"}},
				{"number": 1.5, "field": {"name": "Estimate"}}, {"title": "Iteration 1", "field": {"name": "Iteration"}}]}}
		parsed = board.parse_item(node)
		self.assertEqual((parsed.status, parsed.estimate, parsed.iteration, parsed.size), ("Ready", 1.5, "Iteration 1", None))

	def test_network_hint(self):
		self.assertIn("network access", board.network_hint("Could not resolve host: github.com", "x"))
		self.assertEqual(board.network_hint("HTTP 404: Not Found", "x"), "")
		self.assertIn("network access", board.network_hint("Unknown Owner Type", "x"))

	def test_failed_ls_remote_stops_the_tool(self):
		"""The other fault Copilot found in #447: a failure gave an empty set."""
		real = board.subprocess.run
		board.subprocess.run = lambda cmd, **kw: subprocess.CompletedProcess(
			cmd, 128, "", "fatal: unable to access: Could not resolve host: github.com")
		try:
			with self.assertRaises(SystemExit) as stop:
				board.remote_branches()
		finally:
			board.subprocess.run = real
		self.assertIn("network access", str(stop.exception.code))

	def test_branch_names(self):
		self.assertEqual(board.issue_of("rearch/429-native-and-worker"), 429)
		self.assertIsNone(board.issue_of("rearch/board-priority"))
		self.assertTrue(board.on_stack("rearch/2-x", "rearch/1-y"))
		self.assertFalse(board.on_stack("rearch/2-x", "main"))


class FakeProject:
	def __init__(self):
		self.calls = []

	def add(self, url):
		self.calls.append(("add", url))
		return "NEW"

	def set_status(self, item_id, status):
		self.calls.append(("status", item_id, status))

	def set_iteration(self, item_id):
		self.calls.append(("iteration", item_id))
		return True

	def assign(self, number, login):
		self.calls.append(("assign", number, login))


if __name__ == "__main__":
	unittest.main()
