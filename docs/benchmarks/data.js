window.BENCHMARK_DATA = {
  "lastUpdate": 1776503205701,
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
      }
    ]
  }
}