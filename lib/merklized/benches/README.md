# Merklized Benchmarks

## Usage

```sh
cd 001-state-tree/rs/merklized
cargo bench
```

## Results

```log
test bench_generic_commit_changes_rocksdb_baseline_complex      ... bench:   1,668,108 ns/iter (+/- 370,865)
test bench_generic_commit_changes_rocksdb_baseline_medium       ... bench:     166,773 ns/iter (+/- 25,126)
test bench_generic_commit_changes_rocksdb_baseline_simple       ... bench:      18,195 ns/iter (+/- 2,292)
test bench_generic_commit_changes_rocksdb_jmt_blake3_complex    ... bench:  16,058,075 ns/iter (+/- 2,506,659)
test bench_generic_commit_changes_rocksdb_jmt_blake3_medium     ... bench:   1,051,068 ns/iter (+/- 60,535)
test bench_generic_commit_changes_rocksdb_jmt_blake3_simple     ... bench:      95,576 ns/iter (+/- 5,750)
test bench_generic_commit_changes_rocksdb_jmt_keccak256_complex ... bench:  16,731,066 ns/iter (+/- 3,724,658)
test bench_generic_commit_changes_rocksdb_jmt_keccak256_medium  ... bench:   1,069,602 ns/iter (+/- 95,240)
test bench_generic_commit_changes_rocksdb_jmt_keccak256_simple  ... bench:      98,270 ns/iter (+/- 7,329)
test bench_generic_commit_changes_rocksdb_jmt_sha256_complex    ... bench:  16,693,358 ns/iter (+/- 3,027,495)
test bench_generic_commit_changes_rocksdb_jmt_sha256_medium     ... bench:   1,060,484 ns/iter (+/- 87,908)
test bench_generic_commit_changes_rocksdb_jmt_sha256_simple     ... bench:      98,233 ns/iter (+/- 7,199)

test bench_generic_generate_proof_rocksdb_jmt_blake3_complex    ... bench:      36,105 ns/iter (+/- 1,364)
test bench_generic_generate_proof_rocksdb_jmt_blake3_medium     ... bench:      22,686 ns/iter (+/- 715)
test bench_generic_generate_proof_rocksdb_jmt_blake3_simple     ... bench:      12,326 ns/iter (+/- 319)
test bench_generic_generate_proof_rocksdb_jmt_keccak256_complex ... bench:      36,314 ns/iter (+/- 1,224)
test bench_generic_generate_proof_rocksdb_jmt_keccak256_medium  ... bench:      22,676 ns/iter (+/- 538)
test bench_generic_generate_proof_rocksdb_jmt_keccak256_simple  ... bench:      12,458 ns/iter (+/- 304)
test bench_generic_generate_proof_rocksdb_jmt_sha256_complex    ... bench:      35,899 ns/iter (+/- 632)
test bench_generic_generate_proof_rocksdb_jmt_sha256_medium     ... bench:      22,483 ns/iter (+/- 428)
test bench_generic_generate_proof_rocksdb_jmt_sha256_simple     ... bench:      12,341 ns/iter (+/- 272)

test bench_generic_get_state_root_rocksdb_jmt_blake3_complex    ... bench:       8,234 ns/iter (+/- 388)
test bench_generic_get_state_root_rocksdb_jmt_blake3_medium     ... bench:       8,252 ns/iter (+/- 301)
test bench_generic_get_state_root_rocksdb_jmt_blake3_simple     ... bench:       5,469 ns/iter (+/- 107)
test bench_generic_get_state_root_rocksdb_jmt_keccak256_complex ... bench:       8,303 ns/iter (+/- 343)
test bench_generic_get_state_root_rocksdb_jmt_keccak256_medium  ... bench:       8,257 ns/iter (+/- 324)
test bench_generic_get_state_root_rocksdb_jmt_keccak256_simple  ... bench:       5,436 ns/iter (+/- 163)
test bench_generic_get_state_root_rocksdb_jmt_sha256_complex    ... bench:       8,279 ns/iter (+/- 320)
test bench_generic_get_state_root_rocksdb_jmt_sha256_medium     ... bench:       8,262 ns/iter (+/- 291)
test bench_generic_get_state_root_rocksdb_jmt_sha256_simple     ... bench:       5,410 ns/iter (+/- 169)

test bench_generic_verify_proof_rocksdb_jmt_blake3_complex    ... bench:       6,445 ns/iter (+/- 405)
test bench_generic_verify_proof_rocksdb_jmt_blake3_medium     ... bench:       5,086 ns/iter (+/- 228)
test bench_generic_verify_proof_rocksdb_jmt_blake3_simple     ... bench:       3,963 ns/iter (+/- 164)
test bench_generic_verify_proof_rocksdb_jmt_keccak256_complex ... bench:       6,548 ns/iter (+/- 404)
test bench_generic_verify_proof_rocksdb_jmt_keccak256_medium  ... bench:       5,098 ns/iter (+/- 256)
test bench_generic_verify_proof_rocksdb_jmt_keccak256_simple  ... bench:       3,927 ns/iter (+/- 173)
test bench_generic_verify_proof_rocksdb_jmt_sha256_complex    ... bench:       6,592 ns/iter (+/- 504)
test bench_generic_verify_proof_rocksdb_jmt_sha256_medium     ... bench:       5,275 ns/iter (+/- 296)
test bench_generic_verify_proof_rocksdb_jmt_sha256_simple     ... bench:       3,785 ns/iter (+/- 236)
```
