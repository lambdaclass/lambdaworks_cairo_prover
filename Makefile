.PHONY: test coverage clippy clean

ROOT_DIR:=$(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))

CAIRO0_PROGRAMS_DIR=cairo_programs/cairo0
CAIRO0_PROGRAMS:=$(wildcard $(CAIRO0_PROGRAMS_DIR)/*.cairo)
COMPILED_CAIRO0_PROGRAMS:=$(patsubst $(CAIRO0_PROGRAMS_DIR)/%.cairo, $(CAIRO0_PROGRAMS_DIR)/%.json, $(CAIRO0_PROGRAMS))

# Rule to compile Cairo programs for testing purposes.
# If the `cairo-lang` toolchain is installed, programs will be compiled with it.
# Otherwise, the cairo_compile docker image will be used
# When using the docker version, be sure to build the image using `make docker_build_compiler`.
$(CAIRO0_PROGRAMS_DIR)/%.json: $(CAIRO0_PROGRAMS_DIR)/%.cairo
	@echo "Compiling Cairo program..."
	@cairo-compile --cairo_path="$(CAIRO0_PROGRAMS_DIR)" $< --output $@ 2> /dev/null || \
	docker run --rm -v $(ROOT_DIR)/$(CAIRO0_PROGRAMS_DIR):/pwd/$(CAIRO0_PROGRAMS_DIR) cairo cairo-compile /pwd/$< > $@

build: 
	cargo build --release

run: build
	cargo run --release $(PROGRAM_PATH)
	
test: $(COMPILED_CAIRO0_PROGRAMS)
	cargo test

test_metal: $(COMPILED_CAIRO0_PROGRAMS)
	cargo test -F metal

coverage: $(COMPILED_CAIRO0_PROGRAMS)
	cargo llvm-cov nextest --lcov --output-path lcov.info

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

build_metal:
	cargo b --features metal --release

docker_build_cairo_compiler:
	docker build -f cairo_compile.Dockerfile -t cairo .	
	
docker_compile_cairo:
	docker run -v $(ROOT_DIR):/pwd cairo cairo-compile /pwd/$(PROGRAM) > $(OUTPUT)

target/release/lambdaworks-stark: 
	cargo build --release
	
docker_compile_and_run: target/release/lambdaworks-stark
	@echo "Compiling program with docker"
	@docker run -v $(ROOT_DIR):/pwd cairo cairo-compile /pwd/$(PROGRAM) > $(PROGRAM).json
	@echo "Compiling done \n"
	@cargo run --features instruments --quiet --release $(PROGRAM).json 
	@rm $(PROGRAM).json

compile_and_run: target/release/lambdaworks-stark
	@echo "Compiling program with cairo-compile"
	@cairo-compile $(PROGRAM) > $(PROGRAM).json
	@echo "Compiling done \n"
	@cargo run --features instruments --quiet --release $(PROGRAM).json 
	@rm $(PROGRAM).json

clean:
	rm -f $(CAIRO0_PROGRAMS_DIR)/*.json
	rm -f $(CAIRO0_PROGRAMS_DIR)/*.trace
	rm -f $(CAIRO0_PROGRAMS_DIR)/*.memory

