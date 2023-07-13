# Starknet Stack Prover Lambdaworks
Disclaimer: This prover is still in development and may contain bugs. It is not intended to be used in production yet. We're a few weeks away to have it ready.

## Main building blocks

- [STARKS](https://github.com/lambdaclass/lambdaworks_cairo_prover/tree/main/src/starks): Everything related to STARKs building blocks such as the prover, verifier and FRI.
- [Cairo](https://github.com/lambdaclass/lambdaworks_cairo_prover/tree/main/src/cairo): Implementation of the Cairo AIR.

To be added:

- Grinding ✔️
- Skipping FRI layers
- Optimizing verifier operations
- Range-check ✔️ and Pedersen built-ins
- Different layouts

## Requirements

- Cargo 1.69+
  
## How to try it

For the moment, only programs in Cairo 0 with no arguments and contracts in Cairo 1 with no arguments are supported.

### Prove and verify

To prove Cairo programs you can use:

```bash
make prove PROGRAM_PATH=<compiled_program_path> PROOF_PATH=<output_proof_path>
```

To verify a proof you can use:
  
```bash
make verify PROOF_PATH=<proof_path>
```

For example:

```bash
make prove PROGRAM_PATH=fibonacci.json PROOF_PATH=fibonacci_proof
make verify PROOF_PATH=fibonacci_proof
```

To prove and verify with a single command you can use:

```bash
make run_all PROGRAM_PATH=<proof_path>
```


### Using Docker compiler for Cairo 0 programs

Build the compiler image with:

```bash
make docker_build_cairo_compiler
```

Then for example, if you have a Cairo program in the project folder, you can use:

```bash
make docker_compile_and_run_all PROGRAM=program_name.cairo
```

Or

```bash
make docker_compile_and_prove PROGRAM=program_name.cairo PROOF_PATH=proof_path
```

### Using cairo-compile for Cairo 0 programs

If you have `cairo-lang` installed, you can use it instead of the Dockerfile

Then for example, if you have some Cairo program in the project folder, you can use:

```bash
make compile_and_run_all PROGRAM=program_name.cairo
```

Or 

```bash
make compile_and_prove PROGRAM=program_name.cairo PROOF_PATH=proof_path
```

### Compiling Cairo 1 contracts

Clone `cairo` repository:

``` bash
git clone https://github.com/starkware-libs/cairo
```

Checkout version 1.1.0 (corresponding to that tag of the repository). In the `cairo` folder, run:

``` bash
git checkout v1.1.0
```

- To create json file from Cairo contract:

  ``` bash
  cargo run --bin starknet-compile -- /path/to/input.cairo /path/to/output.json
  ```

- To create casm file from json file:

  ``` bash
  cargo run --bin starknet-sierra-compile -- /path/to/input.json /path/to/output.casm
  ```

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

## Running fuzzers
To run a fuzzer, simply use 

```
make fuzzer <name of the fuzzer>
```

if you don´t have the tools for fuzzing installed use

```
make fuzzer_tools
```

## Benchmarks

To get the results of the table below, run

```
make benchmarks_table
```

The results shown are from the execution of a Fibonacci program.

First table has the results that are independent of the hardware used.

| n   | Trace length | Trace time | Proof size 80 | Proof size 128 |
|-----|--------------|------------|---------------|----------------|
| 100 | 2^10         | 0.9 ms     | 270 KB        | 1.2 MB         |
| 500 | 2^12         | 5.3 ms     | 335 KB        | 1.5 MB         |
| 2k  | 2^14         | 24.7 ms    | 407 KB        | 1.8 MB         |
| 5k  | 2^16         | 77.2 ms    | 488 KB        | 2.2 MB         |
| 20k | 2^18         | 312 ms     | 576 KB        | 2.6 MB         |

Second table has the results of the execution on an Apple M1 with 4 E and 4 P cores and 16 GB of RAM:

<table>
    <tr>
        <th rowspan="2">Trace length</th>
        <th colspan="2" style="text-align:center">Conjecturable 80 bits</th>
        <th colspan="2" style="text-align:center">Provable 128 bits</th>
    </tr>
    <tr>
        <th>Prover time</th>
        <th>Verifier time</th>
        <th>Prover time</th>
        <th>Verifier time</th>
    </tr>
    <tr>
        <td>2^10</td>
        <td>1.1 s</td>
        <td>3.1 ms</td>
        <td>1.1 s</td>
        <td>10.2 ms</td>
    </tr>
    <tr>
        <td>2^12</td>
        <td>335.5 ms</td>
        <td>7.6 ms</td>
        <td>336.4 ms</td>
        <td>16.3 ms</td>
    </tr>
    <tr>
        <td>2^14</td>
        <td>1.41 s</td>
        <td>26.4 ms</td>
        <td>1.42 s</td>
        <td>37 ms</td>
    </tr>
    <tr>
        <td>2^16</td>
        <td>5.8 s</td>
        <td>108.8 ms</td>
        <td>5.8 s</td>
        <td>122.2 ms</td>
    </tr>
    <tr>
        <td>2^18</td>
        <td>24.3 s</td>
        <td>477.4 ms</td>
        <td>24.3 s</td>
        <td>493.1 ms</td>
    </tr>
</table>

Third table has the results of the execution on an Intel Xeon Platinum with 4 cores and 16 GB of RAM:

<table>
     <tr>
        <th rowspan="2">Trace length</th>
        <th colspan="2" style="text-align:center">Conjecturable 80 bits</th>
        <th colspan="2" style="text-align:center">Provable 128 bits</th>
    </tr>
    <tr>
        <th>Prover time</th>
        <th>Verifier time</th>
        <th>Prover time</th>
        <th>Verifier time</th>
    </tr>
    <tr>
        <td>2^10</td>
        <td>2.5 s</td>
        <td>6.3 ms</td>
        <td>2.5 s</td>
        <td>22.4 ms</td>
    </tr>
    <tr>
        <td>2^12</td>
        <td>709 ms</td>
        <td>13.3 ms</td>
        <td>710.5 ms</td>
        <td>33.4 ms</td>
    </tr>
    <tr>
        <td>2^14</td>
        <td>3 s</td>
        <td>41.1 ms</td>
        <td>3 s</td>
        <td>65.8 ms</td>
    </tr>
    <tr>
        <td>2^16</td>
        <td>12.2 s</td>
        <td>160.6 ms</td>
        <td>12.2 s</td>
        <td>190.3 ms</td>
    </tr>
    <tr>
        <td>2^18</td>
        <td>50.5 s</td>
        <td>692.9 ms</td>
        <td>50.5 s</td>
        <td>728.8 ms</td>
    </tr>
</table>
