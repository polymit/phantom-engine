window.BENCHMARK_DATA = {
  "lastUpdate": 1776506927930,
  "repoUrl": "https://github.com/polymit/phantom-engine",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "polymit.main@gmail.com",
            "name": "polymit",
            "username": "polymit-hq"
          },
          "committer": {
            "email": "polymit.main@gmail.com",
            "name": "polymit",
            "username": "polymit-hq"
          },
          "distinct": true,
          "id": "fc0eb5ac97273d886dcb6ebbea6faaddd71f385a",
          "message": "fix(docker): scale down resource limits for CI runner compatibility\n\n- Reduce CPU limit from 8 to 4 to match GitHub Actions hardware.\n- Reduce memory limit from 32G to 6G to stay within runner RAM limits.\n- This allows the 'Full Scale Test' to start without being killed by the Docker daemon.",
          "timestamp": "2026-04-18T14:31:18+05:30",
          "tree_id": "e1d00b578f029158f9ef72674b0454f11322a509",
          "url": "https://github.com/polymit/phantom-engine/commit/fc0eb5ac97273d886dcb6ebbea6faaddd71f385a"
        },
        "date": 1776503205225,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 594,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 601,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 492,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "session_suspend_resume",
            "value": 95,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cct_full_1000_nodes",
            "value": 3252409,
            "range": "± 20835",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3261994,
            "range": "± 122608",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5441,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3162,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "v8_eval_simple",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pool_acquire_tier1",
            "value": 205355,
            "range": "± 1148",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "polymit.main@gmail.com",
            "name": "polymit",
            "username": "polymit-hq"
          },
          "committer": {
            "email": "polymit.main@gmail.com",
            "name": "polymit",
            "username": "polymit-hq"
          },
          "distinct": true,
          "id": "6542879ae5f27fb32c806a574e457c9143bab894",
          "message": "refactor(ci): move scale test to host and cleanup docker execution\n\n- Run internal scale_full_1000 test on the host before Docker build.\n- Remove failing 'cargo test' execution from the slim production container.\n- Ensure the engine logic is verified before spending time on the Docker build.",
          "timestamp": "2026-04-18T14:56:30+05:30",
          "tree_id": "8ba01547f27e54c72e9f2a00a76d33d22452b6b2",
          "url": "https://github.com/polymit/phantom-engine/commit/6542879ae5f27fb32c806a574e457c9143bab894"
        },
        "date": 1776504560804,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 593,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 597,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 491,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "session_suspend_resume",
            "value": 95,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cct_full_1000_nodes",
            "value": 3279970,
            "range": "± 26535",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3271417,
            "range": "± 29520",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5435,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3131,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "v8_eval_simple",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pool_acquire_tier1",
            "value": 204252,
            "range": "± 2028",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "polymit.main@gmail.com",
            "name": "polymit",
            "username": "polymit-hq"
          },
          "committer": {
            "email": "polymit.main@gmail.com",
            "name": "polymit",
            "username": "polymit-hq"
          },
          "distinct": true,
          "id": "3310ebe74e94b9db8a1937d10d90033ec1c4529b",
          "message": "fix(ci): increase rate limits for scale testing\n\n- Set PHANTOM_RATE_LIMIT to 1,000,000 in CI to allow high-concurrency k6 tests.\n- Set PHANTOM_SESSION_LIMIT to 2,000 to ensure the 1,000 session test has headroom.",
          "timestamp": "2026-04-18T15:11:23+05:30",
          "tree_id": "d9175afca43b2a0248f1aaa2db65687ab053ac01",
          "url": "https://github.com/polymit/phantom-engine/commit/3310ebe74e94b9db8a1937d10d90033ec1c4529b"
        },
        "date": 1776505463480,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 561,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 557,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 468,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "session_suspend_resume",
            "value": 81,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cct_full_1000_nodes",
            "value": 2532972,
            "range": "± 103220",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 2514758,
            "range": "± 79482",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 4109,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 2536,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "v8_eval_simple",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pool_acquire_tier1",
            "value": 158990,
            "range": "± 1214",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "polymit.main@gmail.com",
            "name": "polymit",
            "username": "polymit-hq"
          },
          "committer": {
            "email": "polymit.main@gmail.com",
            "name": "polymit",
            "username": "polymit-hq"
          },
          "distinct": true,
          "id": "54110ec302678c8af36d1f3c51d12b598ccb6d1f",
          "message": "refactor(ci): merge docker build and test into single job\n\n- Merge 'docker-build' and 'scale-test-full' into one job.\n- Use 'load: true' to test the image locally without pushing to the registry.\n- Set 'push' to be conditional on tags (v*).\n- This prevents publishing packages on manual runs while still keeping full test coverage.",
          "timestamp": "2026-04-18T15:35:57+05:30",
          "tree_id": "549b9b692dab2c9c35294edcdcfd42197450f876",
          "url": "https://github.com/polymit/phantom-engine/commit/54110ec302678c8af36d1f3c51d12b598ccb6d1f"
        },
        "date": 1776506927437,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 590,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 597,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 490,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "session_suspend_resume",
            "value": 95,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cct_full_1000_nodes",
            "value": 3321873,
            "range": "± 187084",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3319020,
            "range": "± 95607",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5591,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3213,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "v8_eval_simple",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pool_acquire_tier1",
            "value": 205584,
            "range": "± 1856",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}