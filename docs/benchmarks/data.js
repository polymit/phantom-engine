window.BENCHMARK_DATA = {
  "lastUpdate": 1776581758102,
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
          "id": "e39be444ed90fd39899cae0082825f631beab255",
          "message": "fix(engine): link environment variables to limits and increase CPU budget for testing\n\n- Enable PHANTOM_RATE_LIMIT and PHANTOM_SESSION_LIMIT in phantom-mcp server.\n- Refactor McpServer for dynamic limits and updated health reporting.\n- Increase max_cpu_ms_per_sec to 1000ms in phantom-session (NOTE: This increase is temporary and intended for high-scale CI load testing to prevent false budget failures).",
          "timestamp": "2026-04-18T16:22:36+05:30",
          "tree_id": "bacebe70a0041e66aaa5546d1fd0c176493bd33f",
          "url": "https://github.com/polymit/phantom-engine/commit/e39be444ed90fd39899cae0082825f631beab255"
        },
        "date": 1776509727676,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 426,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 438,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 345,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "session_suspend_resume",
            "value": 92,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cct_full_1000_nodes",
            "value": 3050676,
            "range": "± 63585",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3090722,
            "range": "± 10289",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5373,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3104,
            "range": "± 17",
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
            "value": 218079,
            "range": "± 2703",
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
          "id": "054cd5378a83afcfde1962e9c63f3a96d85dbf4c",
          "message": "fix(ci): rename job and pin TAG to current SHA\n\n- Renamed job to 'Build + Full Scale Test (1000 Sessions)'.\n- Explicitly passed TAG env var to docker compose to ensure the fresh build is used.",
          "timestamp": "2026-04-18T16:42:00+05:30",
          "tree_id": "693ae4b8e4e84f1827b464a3bd3df2acb3dca653",
          "url": "https://github.com/polymit/phantom-engine/commit/054cd5378a83afcfde1962e9c63f3a96d85dbf4c"
        },
        "date": 1776510888742,
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
            "value": 604,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 491,
            "range": "± 7",
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
            "value": 3299653,
            "range": "± 26401",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3294055,
            "range": "± 11985",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5419,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3160,
            "range": "± 40",
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
            "value": 205428,
            "range": "± 2578",
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
          "id": "046b2b1ba03634b08282c52fcf7c36e01a1587ba",
          "message": "fix(deploy): make docker-compose image tag dynamic\n\n- Change hardcoded ':latest' to 'latest' to allow CI to test fresh builds.\n- Increase default rate limit to 100,000 to prevent local test bottleneck.",
          "timestamp": "2026-04-18T16:59:26+05:30",
          "tree_id": "fbd5e7004df108ae16b715640abd5845facea6c9",
          "url": "https://github.com/polymit/phantom-engine/commit/046b2b1ba03634b08282c52fcf7c36e01a1587ba"
        },
        "date": 1776511950326,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 558,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 562,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 466,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "session_suspend_resume",
            "value": 80,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "cct_full_1000_nodes",
            "value": 2500247,
            "range": "± 11299",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 2494564,
            "range": "± 10120",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 4190,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 2549,
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
            "value": 155298,
            "range": "± 563",
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
          "id": "0eaf1c28b6623a9b30617393557f8b22e568ee32",
          "message": "fix(deploy): align environment variable names with engine code\n\n- Change PHANTOM_RATE_LIMIT_PER_HOUR to PHANTOM_RATE_LIMIT in docker-compose.yml.\n- This ensures the engine correctly reads the limit from the environment.",
          "timestamp": "2026-04-18T17:01:45+05:30",
          "tree_id": "2c9cffc8d1a0de839869da424983a4b786165fc0",
          "url": "https://github.com/polymit/phantom-engine/commit/0eaf1c28b6623a9b30617393557f8b22e568ee32"
        },
        "date": 1776512075719,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 429,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 440,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 353,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "session_suspend_resume",
            "value": 92,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cct_full_1000_nodes",
            "value": 3105817,
            "range": "± 58196",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3145032,
            "range": "± 52800",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5434,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3217,
            "range": "± 14",
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
            "value": 218282,
            "range": "± 1282",
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
          "id": "843ea6676482a53048a46e3be4d904670b9c7100",
          "message": "fix(ci): harden prometheus metrics verification\n\n- Added retry logic (10 attempts, 5s interval) for Prometheus queries.\n- Improved Python parsing to handle non-JSON or error responses gracefully.\n- Added direct engine metrics check for easier debugging of scrape issues.",
          "timestamp": "2026-04-18T20:35:01+05:30",
          "tree_id": "b2a3c1ec719457cc8a6d460e99f5dd6c04af0711",
          "url": "https://github.com/polymit/phantom-engine/commit/843ea6676482a53048a46e3be4d904670b9c7100"
        },
        "date": 1776524869356,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 605,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 616,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 491,
            "range": "± 4",
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
            "value": 3290385,
            "range": "± 20832",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3276237,
            "range": "± 16703",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5498,
            "range": "± 79",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3249,
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
            "value": 206444,
            "range": "± 1851",
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
          "id": "af190cddbfca391a76e4af0ddf2b7c83ea3a8271",
          "message": "ci: trigger scale test run",
          "timestamp": "2026-04-18T20:57:09+05:30",
          "tree_id": "ab00177bff704b5b016dfc91cbf0bdaf85c43853",
          "url": "https://github.com/polymit/phantom-engine/commit/af190cddbfca391a76e4af0ddf2b7c83ea3a8271"
        },
        "date": 1776526197314,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 590,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 597,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 491,
            "range": "± 4",
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
            "value": 3242534,
            "range": "± 33459",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3228585,
            "range": "± 86060",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5450,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3174,
            "range": "± 16",
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
            "value": 204521,
            "range": "± 1640",
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
          "id": "167a4944809f59cca45a06bcf9a776c71ff936e3",
          "message": "ci: cleanup temporary trigger",
          "timestamp": "2026-04-18T20:58:15+05:30",
          "tree_id": "b2a3c1ec719457cc8a6d460e99f5dd6c04af0711",
          "url": "https://github.com/polymit/phantom-engine/commit/167a4944809f59cca45a06bcf9a776c71ff936e3"
        },
        "date": 1776526256494,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 711,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 717,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 602,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "session_suspend_resume",
            "value": 102,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cct_full_1000_nodes",
            "value": 3228971,
            "range": "± 16204",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3227433,
            "range": "± 14684",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5498,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3272,
            "range": "± 15",
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
            "value": 202331,
            "range": "± 1835",
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
          "id": "e41498c0c019f0253f9b7ccc08cd0a482e9518e6",
          "message": "fix(ci): fix yaml syntax error in metrics verification",
          "timestamp": "2026-04-18T21:01:56+05:30",
          "tree_id": "c29a73937b9af1d4b10131a52d79d311012c79c8",
          "url": "https://github.com/polymit/phantom-engine/commit/e41498c0c019f0253f9b7ccc08cd0a482e9518e6"
        },
        "date": 1776526484601,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 596,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 610,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 491,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "session_suspend_resume",
            "value": 99,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cct_full_1000_nodes",
            "value": 3245388,
            "range": "± 18988",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3234803,
            "range": "± 10531",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5408,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3184,
            "range": "± 34",
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
            "value": 206470,
            "range": "± 1346",
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
          "id": "48689430da9f57127477dcb0e3c5c1e894aca66d",
          "message": "fix(ci): final robust metrics verification using python one-liner",
          "timestamp": "2026-04-18T21:25:19+05:30",
          "tree_id": "5efda9141db41870bedde7904fdbd76a31936910",
          "url": "https://github.com/polymit/phantom-engine/commit/48689430da9f57127477dcb0e3c5c1e894aca66d"
        },
        "date": 1776527887371,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 706,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 710,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 607,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "session_suspend_resume",
            "value": 102,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cct_full_1000_nodes",
            "value": 3234171,
            "range": "± 11990",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3239034,
            "range": "± 22234",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5462,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3256,
            "range": "± 76",
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
            "value": 202526,
            "range": "± 1495",
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
          "id": "3f9e21e43773aeb91f0f876e43c59ba9981afce3",
          "message": "chore: remove duplicate target folder and update gitignore\n\n- Deleted local target folder in phantom-js.\n- Updated .gitignore to recursively ignore all target directories.",
          "timestamp": "2026-04-18T21:57:22+05:30",
          "tree_id": "65b045e2435d741183072593f4b91995bf7ce861",
          "url": "https://github.com/polymit/phantom-engine/commit/3f9e21e43773aeb91f0f876e43c59ba9981afce3"
        },
        "date": 1776529809239,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 594,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 599,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 491,
            "range": "± 1",
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
            "value": 3276521,
            "range": "± 70637",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3256651,
            "range": "± 18685",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5375,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3334,
            "range": "± 15",
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
            "value": 206222,
            "range": "± 4016",
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
          "id": "a8fa22079ec805deb5d339537427fe0e47d5d9f5",
          "message": "chore: make target ignore recursive in .gitignore",
          "timestamp": "2026-04-18T21:58:22+05:30",
          "tree_id": "bb06a17f5b70a39cb881bd990e730db7eaf72bc6",
          "url": "https://github.com/polymit/phantom-engine/commit/a8fa22079ec805deb5d339537427fe0e47d5d9f5"
        },
        "date": 1776529872653,
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
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 494,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "session_suspend_resume",
            "value": 94,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "cct_full_1000_nodes",
            "value": 3273086,
            "range": "± 18329",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3291221,
            "range": "± 82173",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5542,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3279,
            "range": "± 12",
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
            "value": 204961,
            "range": "± 1506",
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
          "id": "fbe5001724bb55d89c0d14b9403561224b3f4c35",
          "message": "fix(security): upgrade rand to 0.9.3 and clean up workspace\n\nThis commit addresses the RUSTSEC-2026-0097 unsoundness vulnerability by\nperforming a surgical upgrade of the rand ecosystem across the workspace.\n\n- Upgraded rand to v0.9.3 and rand_distr to v0.5.0 in anti-detect, js, and net crates.\n- Refactored RngCore trait bounds and OsRng usage for compatibility with rand 0.9.\n- Renamed deprecated thread_rng() calls to rng() to align with modern Rust idioms.\n- Removed RUSTSEC-2026-0097 from audit.toml ignore list.\n- Deleted stray local target/ directories in phantom-serializer and phantom-session.\n\nVerified with cargo check --workspace and cargo audit.",
          "timestamp": "2026-04-19T11:04:32+05:30",
          "tree_id": "47410b872dcb96f1fd6ea32de1b95ec3701fbfae",
          "url": "https://github.com/polymit/phantom-engine/commit/fbe5001724bb55d89c0d14b9403561224b3f4c35"
        },
        "date": 1776577194740,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 589,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 599,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 490,
            "range": "± 4",
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
            "value": 3228589,
            "range": "± 68799",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3235319,
            "range": "± 16419",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5437,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3205,
            "range": "± 41",
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
            "value": 205105,
            "range": "± 2129",
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
          "id": "f5b802a19b0c9ba154fea5adf40c9adb1618c31c",
          "message": "fix(test): resolve rand 0.9 compatibility in workspace tests\n\nThis commit resolves the remaining compilation and policy issues\nfrom the rand 0.9.3 upgrade.\n\n- Replaced OsRng with rand::random() in phantom-net tests and integration tests.\n- Aligned MockStepRng with rand 0.9 RngCore trait (removed try_fill_bytes).\n- Removed stale RUSTSEC-2026-0097 ignore rule from deny.toml.\n- Verified zero-warning state across all workspace targets.\n\nFixes build failures in phantom-net and phantom-anti-detect tests.",
          "timestamp": "2026-04-19T12:23:10+05:30",
          "tree_id": "5b170fcaaa194f525041a31eaadd62ce217537ad",
          "url": "https://github.com/polymit/phantom-engine/commit/f5b802a19b0c9ba154fea5adf40c9adb1618c31c"
        },
        "date": 1776581757805,
        "tool": "cargo",
        "benches": [
          {
            "name": "session_create_quickjs",
            "value": 595,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "session_create_v8",
            "value": 601,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "session_clone_cow",
            "value": 490,
            "range": "± 4",
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
            "value": 3249467,
            "range": "± 27490",
            "unit": "ns/iter"
          },
          {
            "name": "cct_selective_1000_nodes",
            "value": 3245575,
            "range": "± 15992",
            "unit": "ns/iter"
          },
          {
            "name": "cct_delta_10_mutations",
            "value": 5367,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "quickjs_eval_simple",
            "value": 3148,
            "range": "± 8",
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
            "value": 204092,
            "range": "± 2776",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}