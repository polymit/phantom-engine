# Branch Protection Configuration

## main branch

Configure in GitHub: Settings → Branches → Add rule → `main`

Required status check:
- `CI Pass (Required Gate)`

Recommended branch protection settings:
- Require a pull request before merging
- Require at least one approval
- Dismiss stale approvals on new commits
- Require status checks to pass before merging
- Require branches to be up to date before merging
- Require conversation resolution before merging
- Require linear history
- Do not allow bypassing branch protections

## CI gate scope

`CI Pass (Required Gate)` depends on these jobs:
1. `fmt`
2. `clippy`
3. `check`
4. `test`
5. `deny`
6. `audit`
7. `lock-verify`
8. `msrv`
9. `doc`
10. `performance-gate`
11. `scale-smoke`
12. `security-isolation`
