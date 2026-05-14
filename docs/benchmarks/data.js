window.BENCHMARK_DATA = {
  "lastUpdate": 1778734858188,
  "repoUrl": "https://github.com/polymit/phantom-engine",
  "entries": {
    "Phantom Engine Performance Firewall": [
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
          "id": "f0976ecfb581254f8416dd6fdf68c2b4b4a89f65",
          "message": "chore: remove deprecated docs folder after documentation unification",
          "timestamp": "2026-05-14T10:25:36+05:30",
          "tree_id": "6ec3474ada6b097bb287e48821439880f402af60",
          "url": "https://github.com/polymit/phantom-engine/commit/f0976ecfb581254f8416dd6fdf68c2b4b4a89f65"
        },
        "date": 1778734858173,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 436,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 441,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 345,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "session_suspend_resume",
            "value": 90,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cct_full_1000_nodes",
            "value": 5638786,
            "range": "± 66274",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3156124,
            "range": "± 12457",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5254,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3103,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "v8_eval_simple",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "pool_acquire_tier1",
            "value": 218470,
            "range": "± 772",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}