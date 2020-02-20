# Deploying Contracts

## Prerequisites

#### Using binaries (recommended):
* Install [`rustup`](https://rustup.rs/).
* Install the [`casperlabs`](INSTALL.md) package, which contains `casperlabs-client`.

#### Building from source:
* Install [`rustup`](https://rustup.rs/).
* Build the [`casperlabs-client`](BUILD.md#build-the-client).

If you build from source, you will need to add the build directories to your `PATH`, for example:
```powershell
export PATH="<path-to-CasperLabs-repo>/client/target/universal/stage/bin:$PATH"
```
Or you can run the client commands from the root directory of the repo using explicit paths to the binaries.

## Instructions

##### Step 1: Clone the [main repo](https://github.com/CasperLabs/CasperLabs/) to obtain the [example contracts](https://github.com/CasperLabs/CasperLabs/tree/dev/execution-engine/contracts/examples) and set up your toolchain
```shell
git clone git@github.com:CasperLabs/CasperLabs.git
cd CasperLabs/execution-engine
rustup toolchain install $(cat rust-toolchain)
rustup target add --toolchain $(cat rust-toolchain) wasm32-unknown-unknown
```

Source code of contract examples are currently located in `./execution-engine/contracts/examples` directory inside the main repo.

##### Step 2: Build the example contracts
```shell
make build-example-contracts
export COUNTER_DEFINE="$(pwd)/target/wasm32-unknown-unknown/release/counter_define.wasm"
```

##### Step 3: Create an account at [clarity.casperlabs.io](https://clarity.casperlabs.io)

Create an account, which automatically creates a new keypair.  This keypair should be downloaded to the machine where you will deploy contracts.

##### Step 4: Add coins to this account

Add coins to this account using the [faucet](https://clarity.casperlabs.io/#/faucet).

##### Step 5: Deploy `counterdefine.wasm`

```shell
casperlabs-client \
    --host deploy.casperlabs.io \
    deploy \
    --private-key <path-to-private-key> \
    --session $COUNTER_DEFINE \
    --payment-amount 2000000
```

Note: `--payment-amount` is used to define the maximum number of motes to spend on the execution of the deploy. In the example, 2,000,000 is the amount needed to execute the counter define contract. The source code for the contract used in this example can be found [here](https://github.com/CasperLabs/CasperLabs/blob/v0.14.0/execution-engine/contracts/examples/counter-define/src/lib.rs).

You should see the following output:

```shell
Success!
```

Note: The deploy command is a convenience function combining multiple actions (`make`, `sign`,` send`) in the case of a single signature. For signing with multiple keys, see [Advanced usage](#advanced-usage) in this document . 

##### Step 6: Observe

See the instructions [here](QUERYING.md).  


##### Step 7: Call the counter contract

Note: `--payment-amount` is used to define the maximum number of motes to spend on the execution of the deploy.
```shell
casperlabs-client \
    --host deploy.casperlabs.io \
    deploy \
    --private-key <path-to-private-key> \
    --session-name counter_inc \
    --payment-amount 2000000
```

`--session-name` tells the system to use a previous stored contract under the given name. In this case the `counter_define` wasm we deployed in Step 5 stored a contract under the name `counter_inc`, which we can now call.

You should see the following output:
```
Success!
```

##### Step 8: Call a contract with arguments

```shell
export TRANSFER="$(pwd)/target/wasm32-unknown-unknown/release/transfer_to_account.wasm"

casperlabs-client \
    --host deploy.casperlabs.io \
    deploy \
    --private-key <path-to-new-private-key> \
    --session $TRANSFER \
	--session-args '[{"name" : "target", "value" : {"bytes_value" : "<base-16-public-key>"}}, {"name": "amount", "value" : {"long_value" : 1000}}]' \
    --payment-amount 2000000
```

where `<public-key-in-hex>` is the address to send the motes to.

Note: transfers can be done in a more convenient way using the `transfer` sub-command of the client, see `casperlabs-client transfer --help` for details.

## Contract argument details

Smart contracts can be parametrized. A list of contract arguments can be specified on command line when the contract is deployed.

Client's `deploy` command accepts parameter `--session-args` that can be used to specify types and values of contract arguments as a serialized sequence of [Arg](https://github.com/CasperLabs/CasperLabs/blob/v0.14.0/protobuf/io/casperlabs/casper/consensus/consensus.proto#L78) values in a [protobuf JSON format](https://developers.google.com/protocol-buffers/docs/proto3#json), with binary data represented in Base16 format.

Continuing from the example above, see Step 8.

Note: The Contract API `get_arg` function accepts an index. For more detail see Contract API `get_arg` [here](https://docs.rs/casperlabs-contract/0.2.0/casperlabs_contract/contract_api/runtime/index.html).

**Supported types of contract arguments**

| protobuf [Arg](https://github.com/CasperLabs/CasperLabs/blob/v0.14.0/protobuf/io/casperlabs/casper/consensus/consensus.proto#L91) | Contract API type | Example value in [protobuf JSON format](https://developers.google.com/protocol-buffers/docs/proto3#json)
| ---------------  | ------------- | -------------------------------------
| `int_value`      | `u32`         | `'[{"name": "amount", "value": {"int_value": 123456}}]'`
| `long_value`     | `u64`         | `'[{"name": "amount", "value": {"long_value": 123456}}]'`
| `big_int`        | `u512`        | `'[{"name": "amount", "value": {"big_int": {"value": "123456", "bit_width": 512}}}]'`
| `string_value`   | `String`      | `'[{"name": "surname", "value": {"string_value": "Nakamoto"}}]'`
| `optional_value` | `Option<T>`   | `'{"name": "maybe_number", "value": {"optional_value": {}}}` or  `{"name": "maybe_number", "value": {"optional_value": {"long_value": 1000000}}}'`
| `hash`           | `Key::Hash`    | `'{"name": "my_hash", "value": {"key": {"hash": {"hash": "9d39b7fba47d07c1af6f711efe604a112ab371e2deefb99a613d2b3dcdfba414"}}}}'`
| `address`        | `Key::Address` | `'{"name": "my_address", "value": {"key": {"address": {"account": "9d39b7fba47d07c1af6f711efe604a112ab371e2deefb99a613d2b3dcdfba414"}}}}'`
| `uref`           | `Key::URef`    | `'{"name": "my_uref", "value": {"key": {"uref": {"uref": "9d39b7fba47d07c1af6f711efe604a112ab371e2deefb99a613d2b3dcdfba414", "access_rights": 5}}}}'`
| `local`          | `Key::Local`   | `'{"name": "my_local", "value": {"key": {"local": {"hash": "9d39b7fba47d07c1af6f711efe604a112ab371e2deefb99a613d2b3dcdfba414"}}}}'`
| `int_list`       | `Vec<i32>`         | `'{"name": "my_int_list", "value": {"int_list": {"values": [0, 1, 2]}}}'`
| `string_list`    | `Vec<String>`         | `'{"name": "my_string_list", "value": {"string_list": {"values": ["A", "B", "C"]}}}'`

Numeric values of `access_rights` in `uref` are defined in [`enum AccessRights in state.proto](https://github.com/CasperLabs/CasperLabs/blob/v0.14.0/protobuf/io/casperlabs/casper/consensus/state.proto#L144).

## Advanced usage

### Creating, signing, and deploying contracts 

**Deploying contracts with multiple signatures**

To make a deploy signed with multiple keys: first make the deploy with `make-deploy`, sign it with the keys calling `sign-deploy` for each key, and then send it to the node with `send-deploy`.

Every account can associate multiple keys with it and give each a weight. Collective weight of signing keys decides whether an action of certain type can be made. In order to collect weight of different associated keys a deploy has to be signed by corresponding private keys. The `deploy` command creates a deploy, signs it and deploys to the node but doesn't allow for signing with multiple keys. Therefore we split `deploy` into separate commands:

* `make-deploy`  - creates a deploy from input parameters
* `sign-deploy`  - signs a deploy with given private key
* `print-deploy` - prints information of a deploy
* `send-deploy`  - sends a deploy to CasperLabs node
* `show-deploy`  - queries the status of a deploy

Commands read input deploy from both a file (`-i` flag) and STDIN. They can also write to both file and STDOUT.

For more detailed description, use `--help` flag (`casper-client --help`).

See additional details about generating account keys [here](https://github.com/CasperLabs/CasperLabs/blob/v0.14.0/docs/KEYS.md#generating-account-keys) and associated keys and weights [here](https://techspec.casperlabs.io/en/latest/implementation/accounts.html#associated-keys-and-weights).

####Example usage

#####Creating a deploy

```shell
casperlabs-client \
    --host localhost \
    make-deploy \
    --session session-code.wasm \
    --payment payment-code.wasm \
    --from a1130120d27f6f692545858cc5c284b1ef30fe287caef648b0c405def88f543a
```
This will write a deploy in binary format to STDOUT. It's possible to write it to a file, by supplying `-o` argument:
```shell
casperlabs-client \
    --host localhost \
    make-deploy \
    --session session-code.wasm \
    --payment payment-code.wasm \
    --from a1130120d27f6f692545858cc5c284b1ef30fe287caef648b0c405def88f543a
    -o /deploys/deploy_1
```

**Setting Time to Live of a Deploy**

The node will not accept deploys with `deploy.timestamp` greater than some configurable number of milliseconds in the future (relative to its current time). This maximum future time is configurable. Deploys can only go into blocks after their `deploy.timestamp`. 

Specify a duration for which the deploy can be included in a block prior to expiration. This value may be adjusted depending on the tolerance for storing deploys in the deploy buffer for some time before being able to include them in a block.

Use the CasperLabs client `deploy` sub-command. `--ttl-millis` passes the argument set Time to live, Time (in milliseconds) that the deploy will remain valid for. 

```shell
  casperlabs-client\
  	--host deploy.casperlabs.io \
		deploy \
 	 --ttl-millis <arg>
```

Note: If no parameter is specified, a default (defined in the Chainspec - [Genesis block](https://techspec.casperlabs.io/en/latest/theory/naive-blockchain.html#blockdag)) will be used.

**Setting Deploy Dependencies**

This parameter provides a mechanism implemented to explicitly enforce an ordering to deploys. 

Use the CasperLabs client `deploy` sub-command.`--dependencies` passes the argument list of deploy hashes (base16 encoded) which must be executed before this deploy. 

```shell
casperlabs-client\
 	 --host deploy.casperlabs.io \
   deploy \
  	--dependencies <arg>...
```

#####Signing a deploy

```sh
casperlabs-client \
    --host localhost \
    sign-deploy \
    --public-key public-key.pem \
    --private-key private-key.pem
```
This will read a deploy to sign from STDIN and output signed deploy to STDOUT. There are `-i` and `-o` flags for, respectively, reading a deploy from a file and writing signed deploy to a file.

Note that this step may be repeated multiple times to sign a deploy with multiple keys. This feature allows supporting multi-sig transactions out-of-the-box. See [Associated Keys and Weights](https://techspec.casperlabs.io/en/latest/implementation/accounts.html#associated-keys-and-weights) for more information about accounts and associated keys and how they can be used to set up multi-sig security on transactions.

#####Printing a deploy

```shell
casperlabs-client \
    --host localhost \
    print-deploy
```
This will print information of a deploy into STDOUT. There are `--json` and `--bytes-standard` flags for, respectively, using standard JSON vs Protobuf text encoding and standard ASCII-escaped for Protobuf or Base64 for JSON bytes encoding vs custom Base16. The same set of flags also available for all `show-*` and `query-state` commands. 

#####Sending deploy to the node

```shell
casperlabs-client \
    --host localhost \
    send-deploy
```
In the example above there is no `-i` argument, meaning that signed deploy will be read from STDIN.

Reading from STDIN and writing to STDOUT allows for piping output from one command to the input of another one (commands are incomplete for better readability):
```shell
casperlabs-client make-deploy [arguments] | \
casperlabs-client sign-deploy --private-key [private_key] --public-key [public_key] | \
casperlabs-client send-deploy
```
#####Showing deploy status

```shell
casperlabs-client\ 
 --host deploy.casperlabs.io \ 
 --port 40401 show-deploy <deploy-hash>
```

 `show-deploy` allows the user to `View properties of a deploy known by Casper on an existing running node.` One of the properties of a deploy is its status. To view the status of a deploy you can use the `show-deploy` command. This will return a status (pending, processed, finalized, discarded as well as information about its execution (success or error with message), and the block(s) it is included in (if any).

The following lists status' returned:

- `PENDING`
- `PROCESSED`
- `FINALIZED`
- `DISCARDED`

See a description of state provided [here](https://github.com/CasperLabs/CasperLabs/blob/v0.14.0/protobuf/io/casperlabs/casper/consensus/info.proto#L54).

You can also retrieve further information from our platform (APIs, et. al.). See additional details [here](QUERYING.md).

###  Using a local standalone node

If you are testing with a [local standalone node](NODE.md#running-a-single-node), you will need to change the `--host` argument:

```shell
casperlabs-client \
    --host 127.0.0.1 \
    deploy \
    --private-key <path-to-private-key> \
    --session $COUNTER_DEFINE
```

You will also need to explicitly propose after making a deploy (or several deploys), in order for your deploys to be committed:

```shell
casperlabs-client --host 127.0.0.1 propose
```

