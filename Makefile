.PHONY: target/release/trial_sat target/debug/trial_sat test

all: target/release/trial_sat target/debug/trial_sat

target/release/trial_sat:
	cargo build --release

target/debug/trial_sat:
	cargo build

test: target/release/trial_sat
	cd tests && python solve_all_instances.py
