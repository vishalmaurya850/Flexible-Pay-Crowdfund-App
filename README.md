<p>&nbsp;</p>
<p align="center">
<img src="https://github.com/andromedaprotocol/andromeda-core/blob/development/asset/core-logo.png" width=1000>
</p>

A monorepository containing all the contracts and packages related to Andromeda Protocol. Full documentation for all the contracts can be found [here](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/introduction).


|Module| Description| Documentation|
|-------------------------------|---------------------------|-----------------------------|
| [address-list](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/modules/andromeda-address-list)| A module used to whitelist/blacklist a list of addresses to interact with the ADO.|[Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/modules/address-list)|
| [rates](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/modules/andromeda-rates)| A module used to add rates (taxes/royalties) on fund transfers| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/modules/rates)|
| [cw721-bids](https://github.com/andromedaprotocol/andromeda-core/tree/development/contracts/non-fungible-tokens/andromeda-cw721-bids)|Module that can be attached to the cw721 ADO as another way to buy and sell NFTs.|[Gitbook](https://docs.andromedaprotocol.io/andromeda/andromeda-digital-objects/cw721-bids)|
| [receipts](https://docs.andromedaprotocol.io/andromeda/smart-contracts/modules/receipt-contract)| A module that can be attached to ADOs that saves the events of messages.| [Gitbook](https://docs.andromedaprotocol.io/andromeda/smart-contracts/modules/receipt-contract)| 

## Packages

| Contract                                                                                                             | Description                                                                                                                                          |
| -------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| [andromeda_protocol](https://github.com/andromedaprotocol/andromeda-core/tree/development/packages) | Package used to define message types and various utility methods used by Andromeda ADO Contracts.|

### ADO Base

The pacakges also includes the [ado_base](https://github.com/andromedaprotocol/andromeda-core/tree/development/packages/ado-base). Since all our ADOs are built using the same architecture, redundency was inevitable. So we decided to bundle up all the functions/messages/structures that are used by all ADOs into the ado_base which can be referenced by any new ADOs. 

## Development

### Testing

All tests can be run using:

`cargo test --workspace`

### Building

All contracts and packages can be built by running the build script:

`./build_all.sh`

This will build all contract `.wasm` files in to the `artifacts` directory at the project root.

To build a single contract, you need to have [wasm-opt](https://command-not-found.com/wasm-opt)
Then run:

`./build.sh [contract name]` or `./build.sh [catogory name]` 



Examples:

`./build.sh andromda vault` to build the vault contract.
or
`./build.sh finance` to build all contracts under the finance category.

They can also be chained to build multiple directories at the same time:

`./build.sh andromeda_app non-fungible-tokens` to build the app contract and all contracts under the non-fungible-tokens category.

### Formatting

Make sure you run `rustfmt` before creating a PR to the repo. You need to install the `nightly` version of `rustfmt`.

```sh
rustup toolchain install nightly
```

To run `rustfmt`,

```sh
cargo fmt
```

### Linting

You should run `clippy` also. This is a lint tool for rust. It suggests more efficient/readable code.
You can see [the clippy document](https://rust-lang.github.io/rust-clippy/master/index.html) for more information.
You need to install `nightly` version of `clippy`.

#### Install

```sh
rustup toolchain install nightly
```

#### Run

```sh
cargo clippy --all --all-targets -- -D warnings
```
### Creating and Interacting with ADOs

Andromeda is deployed on many of the Cosmos chains. Usually this will require you to set up an environment for each chain. Luckily, Andromeda has built the Andromeda CLI, an all in one tool to build, interact, and manage ADOs and wallets for any of the chains. The CLI documentation can be found [here](https://docs.andromedaprotocol.io/andromeda/andromeda-cli/introduction).

## Licensing

[Terms and Conditions](https://github.com/andromedaprotocol/andromeda-core/blob/development/LICENSE/LICENSE.md)
