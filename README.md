# Stone Prover SDK

Rust SDK for the Starkware Stone prover and verifier.

## Install

To use this SDK, you will need the Stone prover and verifier binaries.
You can either follow the instructions on the [Stone repository](https://github.com/starkware-libs/stone-prover), 
download them from the [latest SDK release](https://github.com/Moonsong-Labs/stone-prover-sdk/releases/latest)
or run the following commands:

```shell
git clone --recurse-submodules https://github.com/Moonsong-Labs/stone-prover-sdk.git
cd stone-prover-sdk
bash scripts/install-stone.sh 
```

This will install the prover and verifier in `${HOME}/.stone` and add this directory to your `PATH`.

## Features

### Prove and verify Cairo programs

The `prover` and `verifier` modules contain thin abstractions on top of the prover and verifier.
They allow the user to prove and verify the execution of any Cairo program from Rust code.

### Execute Cairo programs

The `cairo_vm` module provides utility functions over the [cairo-vm](https://github.com/Moonsong-Labs/cairo-vm)
crate to execute Cairo programs using the Rust Cairo VM.

## Contribute

### Set up the development environment

First, clone the repository and install Stone:

```shell
git clone --recurse-submodules https://github.com/Moonsong-Labs/stone-prover-sdk.git
cd stone-prover-sdk
bash scripts/install-stone.sh
```

This step takes several minutes. The script adds the install directory (`$HOME/.stone` by default) to your `PATH`
for supported shells. Make sure that this is the case:

```shell
which cpu_air_prover
# Should print <install-dir>/cpu_air_prover
```
