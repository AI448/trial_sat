DEV_BINARY=target/debug/trial_sat
RELEASE_BINARY=target/release/trial_sat

.PHONY: format test clean $(DEV_BINARY) $(RELEASE_BINARY)

all: $(DEV_BINARY) $(RELEASE_BINARY)

format:
	cargo fmt

$(DEV_BINARY):
	cargo build --profile=dev

$(RELEASE_BINARY):
	cargo build --profile=release

test: target/release/trial_sat
	cd tests && python solve_all_instances.py

clean:
	cargo clean
