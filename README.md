# Lambdaworks Cairo Prover

We're still moving all the code from [LambdaWorks](https://github.com/lambdaclass/lambdaworks) related to the STARK Cairo prover. 
The CI, the documentation and the GPU code hasn't yet been yet migrated.

Disclaimer: This prover is still in development and may contain bugs. It is not intended to be used in production yet.

## Main building blocks

- [STARKS](https://github.com/lambdaclass/lambdaworks_cairo_prover/tree/main/src/starks): Everything related to STARKs building blocks such as the prover, verifier and FRI.
- [Cairo](https://github.com/lambdaclass/lambdaworks_cairo_prover/tree/main/src/cairo): Implementation of the Cairo AIR.

To be added:
- Grinding
- Skipping FRI layers
- Optimizing verifier operations
- Range-check and Pedersen built-ins
- Different layouts

## Requirements

- Cargo 1.69+
  
## How to try it
For the moment, only programs with no builtins can be verified with the CLI. This
feature will be added soon.

### Using Docker compiler

Build the compiler image with:

`make docker_build_cairo_compiler`

Then for example, if you have a Cairo program in the project folder, you can use:

`make docker_compile_and_run PROGRAM=program_name.cairo`

### Using cairo-compile

If you have `cairo-lang` installed, you can use it instead of the Dockerfile

Then for example, if you have some Cairo program in the project folder, you can use:

`make compile_and_run PROGRAM=program_name.cairo`

## Running tests
To run tests, simply use
```
make test
```
If you have the `cairo-lang` toolchain installed, this will compile the Cairo programs needed
for tests.
If you have built the cairo-compile docker image, that will be used for compiling instead.

Be sure to build the docker image if you don't want to install the `cairo-lang` toolchain:
```
make docker_build_cairo_compiler
```
