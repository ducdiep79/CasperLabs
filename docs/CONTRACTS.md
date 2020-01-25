# Deploying Contracts


## Prerequisites

#### Using binaries (recommended):
* Install [`rustup`](https://rustup.rs/).
* Install the [`casperlabs`](INSTALL.md) package, which contains `casperlabs-client`.

#### Building from source:
* Install [`rustup`](https://rustup.rs/).
* Build the [`casperlabs-client`](BUILD.md#build-the-client).

If you build from source, you will need to add the build directories to your `PATH`.

For example:
```
export PATH="<path-to-CasperLabs-repo>/client/target/universal/stage/bin:$PATH"
```
Or you can run the client commands from the root directory of the repo, using explicit paths to the binaries.

## Instructions

##### Step 1: Clone the [main repo](https://github.com/CasperLabs/CasperLabs/tree/master) to obtain the [example contracts](https://github.com/CasperLabs/CasperLabs/tree/dev/execution-engine/contracts/examples) and set up your toolchain
```
git clone git@github.com:CasperLabs/CasperLabs.git
cd CasperLabs/execution-engine
rustup toolchain install $(cat rust-toolchain)
rustup target add --toolchain $(cat rust-toolchain) wasm32-unknown-unknown
```

Source code of contract examples is currently located in the `./execution-engine/contracts/examples` directory inside the [main repo](https://github.com/CasperLabs/CasperLabs/tree/master/).

##### Step 2: Build the example contracts
```
make build-example-contracts
export COUNTER_DEFINE="$(pwd)/target/wasm32-unknown-unknown/release/counter_define.wasm"
export COUNTER_CALL="$(pwd)/target/wasm32-unknown-unknown/release/counter_call.wasm"
```

##### Step 3: Create an account at [clarity.casperlabs.io](https://clarity.casperlabs.io)

Create an account, which automatically creates a new keypair.  This keypair should be downloaded to the machine where you will deploy contracts.

##### Step 4: Add coins to the account

You can add coins to the account using the [faucet](https://clarity.casperlabs.io/#/faucet).

##### Step 5: Deploy `counterdefine.wasm`

Note: `--payment-amount` is used to define the maximum number of motes to spend on the execution of the deploy.
```
casperlabs-client \
    --host deploy.casperlabs.io \
    deploy \
    --private-key <path-to-private-key> \
    --session $COUNTER_DEFINE
    --payment-amount <int>
```

You should see the following output:
```
Success!
```

##### Step 6: Observe

See the instructions [here](QUERYING.md).


##### Step 7: Deploy `countercall.wasm`

Note: `--payment-amount` is used to define the maximum number of motes to spend on the execution of the deploy.
```
casperlabs-client \
    --host deploy.casperlabs.io \
    deploy \
    --private-key <path-to-private-key> \
    --session $COUNTER_CALL
    --payment-amount <int>
```

You should see the following output:
```
Success!
```

###### Alternative way of creating, signing, and deploying contracts

Every account can associate multiple keys with it and give each key a weight. The collective weight of signing keys decides whether an action of a certain type can be made. In order to collect the weight of different associated keys, a deploy has to be signed by corresponding private keys.

The `deploy` command is a convenience function combining multiple actions (make, sign, send) in the case of a single signature, but it does not allow for signing with multiple keys.

To make a deploy signed with multiple keys: first make the deploy with `make-deploy`, sign it with the keys calling `sign-deploy` for each key, and then send it to the node with `send-deploy`.

The following lists commands you can use when deploying contracts:

* `make-deploy`  - creates a deploy from input parameters
* `sign-deploy`  - signs a deploy with given private key
* `print-deploy` - prints information of a deploy
* `send-deploy`  - sends a deploy to CasperLabs node
* `show-deploy`  - queries the status of a deploy (pending, success, error, etc.)

Commands read input deploy from both a file (`-i` flag) and STDIN. They can also write to both file and STDOUT.

Example usage:

**Creating a deploy**
```
casperlabs-client \
    --host localhost \
    make-deploy \
    --session session-code.wasm \
    --payment payment-code.wasm \
    --from a1130120d27f6f692545858cc5c284b1ef30fe287caef648b0c405def88f543a
```
This will write a deploy in binary format to STDOUT. It's possible to write it to a file, by supplying `-o` argument:
```
casperlabs-client \
    --host localhost \
    make-deploy \
    --session session-code.wasm \
    --payment payment-code.wasm \
    --from a1130120d27f6f692545858cc5c284b1ef30fe287caef648b0c405def88f543a
    -o /deploys/deploy_1
```

**Signing a deploy**
```
casperlabs-client \
    --host localhost \
    sign-deploy \
    --public-key public-key.pem \
    --private-key private-key.pem
```
This will read a deploy to sign from STDIN and output signed deploy to STDOUT. There are `-i` and `-o` flags for, respectively, reading a deploy from a file and writing signed deploy to a file.

**Printing a deploy**
```
casperlabs-client \
    --host localhost \
    print-deploy
```
This will print information of a deploy into STDOUT. There are `--json` and `--bytes-standard` flags for, respectively, using standard JSON vs Protobuf text encoding and standard ASCII-escaped for Protobuf or Base64 for JSON bytes encoding vs custom Base16. The same set of flags is also available for all `show-*` and `query-state` commands.

**Sending deploy to the node**
```
casperlabs-client \
    --host localhost \
    send-deploy
```
In the example above there is no `-i` argument, meaning that the signed deploy will be read from STDIN.

Reading from STDIN and writing to STDOUT allows for piping output from one command to the input of another one (commands are incomplete for better readability):
```
casperlabs-client make-deploy [arguments] | \
casperlabs-client sign-deploy --private-key [private_key] --public-key [public_key] |\
casperlabs-client send-deploy
```

For more detailed description about deploy commands, use the `--help` flag (`casper-client --help`).


##### Step 8: Observe

See the instructions [here](QUERYING.md).

**Showing deploy status**

To view the status of a deploy you can use the `show-deploy` command.

For example:
```
casperlabs-client\ 
        --host deploy.casperlabs.io \ 
        --port 40401 show-deploy <deploy-hash>
```

This will return a status (pending, processed, finalized, discarded as well as information about its execution (success or error with message), and the block(s) it is included in (if any).

The following lists status' returned:

* `PENDING`
* `PROCESSED`
* `FINALIZED`
* `DISCARDED`

See a description of state provided [here](https://github.com/CasperLabs/CasperLabs/blob/907c46b2c7dc36ad8944b1cd104238122dc2e4ad/protobuf/io/casperlabs/casper/consensus/info.proto#L54).

You can also retrieve further information from our platform (with our APIs, et. al.). See additional details [here](QUERYING.md).

###### Advanced deploy options

**Stored contracts**

A function that is part of the deployed contract's module can be saved on the blockchain with the Contract API function `store_function`.

Such function becomes a stored contract that can be later called from another contract with `call_contract`, or used instead of a WASM file when creating a new deploy on the command line.

**Contract address**

The `store_function` stores under a `URef`, while `store_function_at_hash` stores under a `Hash` -- a 256-bit unique identifier which is generated by the system.

It is important to note that when a contract is stored under a `Hash` it is immutable (that `Hash` will always point to exactly that contract), while storing under a `URef` allows the contract to be upgraded with the `upgrade_contract_at_uref` function.

**Calling a stored contract using its address**

Contract address is a cryptographic hash uniquely identifyiyng a stored contract in the system. Thus, it can be used to call the stored contract directly when creating a deploy, for example on command line or from another contract.

The `casperlabs-client` `deploy` command accepts argument `--session-hash`, which can be used to create a deploy using a stored contract instead of a file with a compiled WASM module. Its value should be a base16 representation of the contract address.

For example:

`--session-hash 2358448f76c8b3a9e263571007998791a815e954c3c3db2da830a294ea7cba65`.

`--payment-hash` is an option equivalent to `--session-hash`
but for specifying the address of a payment contract.

**Calling a stored contract by name**

For convenience, a contract address can be associated with a name in the context of a user's account. Typically this is done in the same contract that calls `store_function`.

In the example below, `counter_ext` is a function in the same module as the executing contract. The function is stored on blockchain with `store_function`. Next, a call to `put_key` associates the stored contract's address with a name `"counter"`.

For example:
```
    //create map of references for stored contract
    let mut counter_urefs: BTreeMap<String, Key> = BTreeMap::new();
    let pointer = store_function("counter_ext", counter_urefs);
    put_key("counter", &pointer.into());
```


`casperlabs-client` `deploy` command accepts argument `--session-name` which can be used to refer to a stored contract by its name.

For example:
```
--session-name counter
```
This option can be used to create a deploy with a stored contract
acting as the deploy's session contract. An equivalent argument for a payment contract is `--payment-name`.

Note: names are valid only in the context of the account which called `put_key`.


**Calling a stored contract by reference**

In the example below, `COUNTER_EXT` is a function in the same module as the executing contract. The function is stored on blockchain with `store_function_at_hash`. Next, a call to `put_key` associates the stored contract's address with a `COUNTER_KEY`.

```
//create map of references for stored contract
    let mut counter_urefs: BTreeMap<String, Key> = BTreeMap::new();
    let key_name = String::from(COUNT_KEY);
    counter_urefs.insert(key_name, counter_local_key.into());
    let pointer = storage::store_function_at_hash(COUNTER_EXT, counter_urefs);
    put_key(COUNTER_KEY, pointer.into());

```

**Understanding the difference between calling a contract directly and with `call_contract`**

When a contract is stored with `store_function`  there is a new context created for it, with initial content defined by the map passed to `store_function` as its second argument. Later, when the stored contract is called with `call_contract` it is executed in this context.

In contrast, when the same stored contract is called directly,
for example its address is passed to `--session-hash` argument of the `deploy` command, the contract will be executed in the context of the account that creates the deploy. The consequence of this is that stateful contracts designed to operate in a specific context may not work as expected when called directly. They may, for instance, attempt to read or modify a `URef` that they expect to exist in their context, but find it missing in the context that they are actually run in -- that is of the deployer's account.

Note that for now, one way to get around this (as this will likely change in the future), is to create a stateless "proxy" contract which essentially does `call_contract` and nothing else.

For example in [this](https://github.com/CasperLabs/CasperLabs/tree/dev/execution-engine/contracts/examples/counter-define#deploy) contract: Increment the counter
```
$ casperlabs-client --host $HOST deploy \
    --private-key $PRIVATE_KEY_PATH \
    --payment-amount 10000000 \
    --session-name counter_inc
```
The counter contract example, stores an additional function for incrementing the counter. This function can be called directly by name in order to increment the counter without sending additional wasm.

**Passing arguments to contracts**

Smart contracts can be parametrized. A list of contract arguments can be specified on command line when the contract is deployed.

When the contract code is executed it can access individual arguments by calling Contract API function `get_arg` with index of an argument. First argument is indexed with `0`.

**Time to Live and Deploy Dependency**

**Time to Live** specifies a duration for which the deploy can be included in a block prior to expiration. The node will not accept deploys with `deploy.timestamp` greater than some configurable number of milliseconds in the future (relative to its current time). This maximum future time is configurable -- this parameter is not at the protocol level since deploys can only go into blocks after their `deploy.timestamp`. Since individual nodes choose their own future cut-off, this is set by node operators, not users.

This value may be adjusted depending on the tolerance for storing deploys in the deploy buffer for some time before being able to include them in a block.

Use the CasperLabs client `deploy` sub-command.

For example:
```
casperlabs-client\
    --host deploy.casperlabs.io \
        deploy \
    --ttl-millis  <arg>
```

`--ttl-millis` passes the argument set Time to live, Time (in milliseconds) that the deploy will remain valid for. If no parameter is specified, a default (defined in the Chainspec - Genesis block) will be used.

**Deploy Dependency** provides a parameter for a mechanism implemented to explicitly enforce an ordering to deploys. This is important since sometimes order matters.

Use the CasperLabs client `deploy` sub-command.

For example:
```
casperlabs-client\
    --host deploy.casperlabs.io \
        deploy \
    --dependencies  <arg>...
```

 `--dependencies`  passes the argument list of deploy hashes (base16 encoded) which must be executed before this deploy.

**Command line client's syntax of contract arguments**

Client's `deploy` command accepts parameter `--session-args`
that can be used to specify types and values of contract arguments as a serialized sequence of [Arg](https://github.com/CasperLabs/CasperLabs/blob/ca35f324179c93f0687ed4cf67d887176525b73b/protobuf/io/casperlabs/casper/consensus/consensus.proto#L78) values in a [protobuf JSON format](https://developers.google.com/protocol-buffers/docs/proto3#json), with binary data represented in Base16 format.

For example:

`--session-args '[{"name": "amount", "value": {"long_value": 123456}}]'`.


**Accessing arguments in contracts**

The Contract API function `get_arg` allows you to access contract arguments via their indices `(get_args(0))`.

For example:
```
get_arg("amount")
```
```
let amount: u64 = get_arg(0);
```
This will deserialize the contract argument `amount` as a value of type `u64`.

Note, types of the arguments specified when deploying and the types in the Rust code must match. The matching type for protobuf [Arg](https://github.com/CasperLabs/CasperLabs/blob/ca35f324179c93f0687ed4cf67d887176525b73b/protobuf/io/casperlabs/casper/consensus/consensus.proto#L78) type `long_value` is currently `u64`.

The same can be achieved by declaring return type of `get_arg` explicitly.

For example:

```
let amount = get_arg::<u64>(0);
```



**Supported types of contract arguments**


| protobuf [Arg](https://github.com/CasperLabs/CasperLabs/blob/ca35f324179c93f0687ed4cf67d887176525b73b/protobuf/io/casperlabs/casper/consensus/consensus.proto#L78) | Contract API type | Example value in [protobuf JSON format](https://developers.google.com/protocol-buffers/docs/proto3#json)
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

Numeric values of `access_rights` in `uref` are defined in
[`enum AccessRights in state.proto](https://github.com/CasperLabs/CasperLabs/blob/ca35f324179c93f0687ed4cf67d887176525b73b/protobuf/io/casperlabs/casper/consensus/state.proto#L58).

####  Using a local standalone node

If you are testing with a [local standalone node](NODE.md#running-a-single-node), you will need to change the `--host` argument:

```
casperlabs-client \
    --host 127.0.0.1 \
    deploy \
    --private-key <path-to-private-key> \
    --session $COUNTER_DEFINE
```

You will also need to explicitly propose after making a deploy (or several deploys), in order for your deploys to be committed:

```
casperlabs-client --host 127.0.0.1 propose
```
