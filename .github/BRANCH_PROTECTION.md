# Branch Protection Configuration

## `main` branch

Configure in GitHub: `Settings -> Branches -> Add rule -> main`

Required status check:
- `CI Pass (Required Gate)`

Additional settings:
- Require a pull request before merging
- Require status checks to pass before merging
- Require branches to be up to date before merging (strict)
- Require conversation resolution before merging
- Require linear history
- Do not allow bypassing the above settings

## What `CI Pass` enforces

`CI Pass` succeeds only when all 12 jobs pass:
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

Missing any one of these blocks merges to `main`.
