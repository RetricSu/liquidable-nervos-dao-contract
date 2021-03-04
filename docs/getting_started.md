## Getting Started

### Pre-requirement

- [capsule](https://github.com/nervosnetwork/capsule) >= 0.4.3
- [ckb-cli](https://github.com/nervosnetwork/ckb-cli) >= 0.35.0
- [secp256k1_blake2b_sighash_all_dual](https://github.com/nervosnetwork/ckb-miscellaneous-scripts/blob/master/c/secp256k1_blake2b_sighash_all_dual.c) which supports loaded as a shared library.

> Note: Capsule uses [docker](https://docs.docker.com/get-docker/) to build contracts and run tests. docker and ckb-cli must be accessible in the PATH in order for them to be used by Capsule.

### Development

Git clone this repo & cd into dir

```sh
git clone https://github.com/RetricSu/liquidable-nervos-dao-contract.git

cd liquidable-nervos-dao-contract
```

Init submodules:

```sh
git submodule init && git submodule update -r --init
```

Build the shared binary secp256k1_blake2b_sighash_all_dual:

```sh
cd ckb-miscellaneous-scripts && git submodule init && git submodule update

make all-via-docker
```

Build contracts:

``` sh
capsule build
```

Run tests:

``` sh
capsule test
```

### Deployment

Build release version of script

```sh
capsule build --release
```

Deploy the script

```sh
capsule deploy --address <ckt1....> --fee 0.001
```