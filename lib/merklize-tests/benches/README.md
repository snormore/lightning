# Merklize Benchmarks

## Usage

```sh
cd 001-state-tree/rs/merklize
cargo bench
```

## Results

```log
test bench_application_commit_changes_rocksdb_baseline_complex      ... bench:   4,535,225.00 ns/iter (+/- 291,308.37)
test bench_application_commit_changes_rocksdb_baseline_medium       ... bench:     199,197.92 ns/iter (+/- 2,429.80)
test bench_application_commit_changes_rocksdb_baseline_simple       ... bench:      15,714.73 ns/iter (+/- 538.53)
test bench_application_commit_changes_rocksdb_jmt_keccak256_complex ... bench:   4,597,435.45 ns/iter (+/- 253,580.60)
test bench_application_commit_changes_rocksdb_jmt_keccak256_medium  ... bench:     241,759.12 ns/iter (+/- 9,697.17)
test bench_application_commit_changes_rocksdb_jmt_keccak256_simple  ... bench:      53,376.25 ns/iter (+/- 4,507.84)

test bench_generic_commit_changes_rocksdb_baseline_complex      ... bench:   1,619,472.90 ns/iter (+/- 66,359.28)
test bench_generic_commit_changes_rocksdb_baseline_medium       ... bench:     159,020.18 ns/iter (+/- 21,581.24)
test bench_generic_commit_changes_rocksdb_baseline_simple       ... bench:      16,203.02 ns/iter (+/- 904.94)
test bench_generic_commit_changes_rocksdb_jmt_blake3_complex    ... bench:  14,786,958.40 ns/iter (+/- 1,882,652.39)
test bench_generic_commit_changes_rocksdb_jmt_blake3_medium     ... bench:     952,010.45 ns/iter (+/- 35,683.78)
test bench_generic_commit_changes_rocksdb_jmt_blake3_simple     ... bench:      90,981.67 ns/iter (+/- 2,616.96)
test bench_generic_commit_changes_rocksdb_jmt_keccak256_complex ... bench:  14,808,504.20 ns/iter (+/- 2,130,626.74)
test bench_generic_commit_changes_rocksdb_jmt_keccak256_medium  ... bench:     957,627.10 ns/iter (+/- 38,285.46)
test bench_generic_commit_changes_rocksdb_jmt_keccak256_simple  ... bench:      92,236.01 ns/iter (+/- 7,410.97)
test bench_generic_commit_changes_rocksdb_jmt_sha256_complex    ... bench:  14,681,987.40 ns/iter (+/- 1,708,312.94)
test bench_generic_commit_changes_rocksdb_jmt_sha256_medium     ... bench:   1,015,002.10 ns/iter (+/- 191,163.95)
test bench_generic_commit_changes_rocksdb_jmt_sha256_simple     ... bench:      94,603.01 ns/iter (+/- 4,857.35)

test bench_generic_generate_proof_rocksdb_jmt_blake3_complex    ... bench:      30,807.86 ns/iter (+/- 1,012.80)
test bench_generic_generate_proof_rocksdb_jmt_blake3_medium     ... bench:      19,953.27 ns/iter (+/- 1,085.73)
test bench_generic_generate_proof_rocksdb_jmt_blake3_simple     ... bench:      11,132.13 ns/iter (+/- 527.87)
test bench_generic_generate_proof_rocksdb_jmt_keccak256_complex ... bench:      30,663.62 ns/iter (+/- 884.44)
test bench_generic_generate_proof_rocksdb_jmt_keccak256_medium  ... bench:      19,790.31 ns/iter (+/- 763.17)
test bench_generic_generate_proof_rocksdb_jmt_keccak256_simple  ... bench:      11,150.11 ns/iter (+/- 493.59)
test bench_generic_generate_proof_rocksdb_jmt_sha256_complex    ... bench:      30,751.24 ns/iter (+/- 868.78)
test bench_generic_generate_proof_rocksdb_jmt_sha256_medium     ... bench:      19,593.08 ns/iter (+/- 386.00)
test bench_generic_generate_proof_rocksdb_jmt_sha256_simple     ... bench:      11,251.01 ns/iter (+/- 294.28)

test bench_generic_get_state_root_rocksdb_jmt_blake3_complex    ... bench:       6,890.79 ns/iter (+/- 284.64)
test bench_generic_get_state_root_rocksdb_jmt_blake3_medium     ... bench:       6,944.30 ns/iter (+/- 179.48)
test bench_generic_get_state_root_rocksdb_jmt_blake3_simple     ... bench:       4,754.51 ns/iter (+/- 160.27)
test bench_generic_get_state_root_rocksdb_jmt_keccak256_complex ... bench:       6,907.99 ns/iter (+/- 135.03)
test bench_generic_get_state_root_rocksdb_jmt_keccak256_medium  ... bench:       6,864.30 ns/iter (+/- 242.21)
test bench_generic_get_state_root_rocksdb_jmt_keccak256_simple  ... bench:       4,778.17 ns/iter (+/- 101.20)
test bench_generic_get_state_root_rocksdb_jmt_sha256_complex    ... bench:       6,871.63 ns/iter (+/- 319.86)
test bench_generic_get_state_root_rocksdb_jmt_sha256_medium     ... bench:       6,860.85 ns/iter (+/- 92.17)
test bench_generic_get_state_root_rocksdb_jmt_sha256_simple     ... bench:       4,731.89 ns/iter (+/- 105.08)

test bench_generic_verify_proof_rocksdb_jmt_blake3_complex    ... bench:       6,326.34 ns/iter (+/- 531.76)
test bench_generic_verify_proof_rocksdb_jmt_blake3_medium     ... bench:       4,957.40 ns/iter (+/- 433.89)
test bench_generic_verify_proof_rocksdb_jmt_blake3_simple     ... bench:       3,810.67 ns/iter (+/- 231.63)
test bench_generic_verify_proof_rocksdb_jmt_keccak256_complex ... bench:       6,471.22 ns/iter (+/- 412.15)
test bench_generic_verify_proof_rocksdb_jmt_keccak256_medium  ... bench:       4,893.66 ns/iter (+/- 282.71)
test bench_generic_verify_proof_rocksdb_jmt_keccak256_simple  ... bench:       3,815.62 ns/iter (+/- 218.01)
test bench_generic_verify_proof_rocksdb_jmt_sha256_complex    ... bench:       6,176.00 ns/iter (+/- 212.81)
test bench_generic_verify_proof_rocksdb_jmt_sha256_medium     ... bench:       4,854.13 ns/iter (+/- 105.96)
test bench_generic_verify_proof_rocksdb_jmt_sha256_simple     ... bench:       3,822.70 ns/iter (+/- 174.68)
```
