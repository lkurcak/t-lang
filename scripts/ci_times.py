#!/usr/bin/env python3
"""How long CI's test jobs take, from GitHub's records (needs `gh`).

    python3 scripts/ci_times.py [N]      the test jobs of the last N runs on main
    python3 scripts/ci_times.py RUN_ID   each test binary in that run's test jobs

The first is the trend: each successful push to main whose tests ran, newest
first, with each platform's test job in minutes. The second is where one
run's time went: cargo runs the test binaries one after another, so their
times add up to the job's, less the build.
"""

import json
import re
import subprocess
import sys
from datetime import datetime

REPO = 'lkurcak/t-lang'


def gh(*args):
    return subprocess.run(['gh', *args], check=True, capture_output=True, text=True).stdout


def jobs(run):
    data = json.loads(gh('api', f'repos/{REPO}/actions/runs/{run}/jobs?per_page=100'))
    return [j for j in data['jobs'] if j['name'].startswith('ci / test (') and j['conclusion'] == 'success']


def minutes(job):
    start, end = (datetime.fromisoformat(job[k].replace('Z', '+00:00')) for k in ('started_at', 'completed_at'))
    return (end - start).total_seconds() / 60


def platform(job):
    return job['name'][len('ci / test ('):-1].removesuffix('-latest')


def trend(count):
    runs = json.loads(gh('run', 'list', '--repo', REPO, '--workflow', 'ci.yml', '--branch', 'main',
                         '--event', 'push', '--status', 'success', '--limit', str(count * 2),
                         '--json', 'databaseId,createdAt,headSha'))
    shown = 0
    for run in runs:
        tested = jobs(run['databaseId'])
        if not tested:  # a change to the backlog alone runs no tests
            continue
        times = '  '.join(f'{platform(j)} {minutes(j):4.1f}' for j in sorted(tested, key=platform))
        print(f"{run['createdAt'][:16].replace('T', ' ')}  {run['headSha'][:7]}  {times}")
        shown += 1
        if shown == count:
            break


def binaries(run):
    for job in sorted(jobs(run), key=platform):
        log = gh('run', 'view', '--repo', REPO, '--job', str(job['id']), '--log')
        # The k-th result is the k-th binary's: cargo's lines (stderr) and the
        # tests' (stdout) can come out of order, but each in its own order.
        names, results, build = [], [], '?'
        for line in log.splitlines():
            line = re.sub(r'(\x1b|\^\[)\[[0-9;]*m', '', line)
            if m := re.search(r'Running (?:unittests )?(\S+)', line):
                names.append(m.group(1))
            elif m := re.search(r'Doc-tests (\S+)', line):
                names.append(f'doc-tests {m.group(1)}')
            elif (m := re.search(r'Finished .* in (.+)$', line)) and build == '?':
                build = m.group(1).strip()
            elif m := re.search(r'test result: \S+ (\d+) passed.* finished in ([\d.]+)s', line):
                results.append((float(m.group(2)), int(m.group(1))))
        rows = [(seconds, tests, name) for (seconds, tests), name in zip(results, names)]
        total = sum(r[0] for r in rows) / 60
        print(f'{platform(job)}: job {minutes(job):.1f} min, build {build}, '
              f'{len(rows)} test binaries {total:.1f} min')
        for seconds, tests, name in sorted(rows, reverse=True)[:12]:
            print(f'  {seconds:6.1f} s  {tests:3d} tests  {name}')


if __name__ == '__main__':
    arg = sys.argv[1] if len(sys.argv) > 1 else '15'
    if len(arg) > 6:
        binaries(arg)
    else:
        trend(int(arg))
