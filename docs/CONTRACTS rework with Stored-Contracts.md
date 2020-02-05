

# Stored Contracts Title Stub (for placement only)

## Instructions title stub (for placement only)

https://casperlabs.atlassian.net/wiki/spaces/REL/pages/134545929/Node+0.7+Release+Plan

key features for efficiency and  ease of operation 

- dApp developers and 

- node operators. 

dApp developers can call an existing contract on the blockchain in the following ways:

- using its address (without sending any Wasm code in the deploy) and 
- by sending the arguments to the contract. 

~~This will minimize deploy size, ergo block size thus reducing the bandwidth required and increasing throughput.~~ 

Our Flexible interface for the dApp developers includes:

- the Scala client which has been updated to take both the contract WASM bytes and a list of arguments in Proto 3 JSON format. 
-  `Standard Payment Code`  is used as the default where no payment code is supplied for the deploy. 
- Persistent deploy buffers
-  **"Voting matrix"**, a high-performance implementation of "The Inspector" finality criterion that **helps to identify finality of a block more efficiently (currently supports only acknowledgment level 1**).
-  basic deploy selection strategy with a block size limit.

It is not necessary the calling dApp (first) hard-code arguments in the contracts (and then) precompile the contract (with arguments) into WASM in order to deploy a smart contract for `payment code` and `session code`; hardcoding arguments does not promote flexible application structure, and the need to continually re-deploy the same basic contract is a waste of space on the chain.

 **usability**

The deploy command ~~can now~~ accepts a list of arguments for either (or both) of the payment and session contracts, 

as values of parameters `--session-args` and `--payment-args`.  

~~(This flexible interface for dApp developers to use, with the original interface  remaining.)~~ 

Additionally, **payment and session contracts ~~previously~~ stored on the chain** can be called by providing the 

- `hash`, 

- `uref`, or 

- `name` of the contract.   

This cuts down on the amount of WASM passed and the amount of transaction data stored.  

The following is a list of parameters now available in the deploy subcommand: 

**Subcommand Deploy - Constructs a Deploy and sends it**







## Stored contracts

There are two ways to store a contract under a Key: both correspond to 

`--session-name` or `--payment-name`

- `store_function` stores a contract under a `Key::URef`
- `store_function_at_hash` stores a contract under a `Key::Hash`

## Associating a Key with a human-readable name:

To associate a Key with a human-readable name: use `put_key`. This association is only valid in the context where `put_key` is run.

## Ways to execute code from the client (e.g., scala or python):

- Provide the wasm directly, this corresponds to `--session` or `--payment` depending on what execution phase you want the code to be used for, 

  For example:

  `fpo`          

- Provide the `Key::Hash address` in `base-16` that a contract was previously stored under, the `store_function_at_hash`, this corresponds to `--session-hash` or `--payment-hash`.

​       For example:

`    fpo                 `

- Provide the human readable name that is associated with some Key  `put_key`  a contract was previously stored under via either `store_* function` (i.e. `store_function`, `store_function_at_hash`)`,  this corresponds to`--session-name` or  `--payment-name`.

   For example:

  `    fpo   `

- Provide the bytes identifying the `Key::URef` that a contract was previously stored under (this corresponds to `--session-uref` or `--payment-uref`)

   For example:

  `    fpo   `


Note that if you don't use the human readable name, you will get a forged `URef error`. You must have first called the `put_key` to persist the `URef` in the account, in which case you would have used the key 





### Rust Examples

For example: 

 `store_function` 

 For example: 

 `store_function_at_hash`


  ### Client CLI examples

  For example:  

  `--session-hash` (which only works with contracts stored at hashes of course) 

  `--session-name` -- works with either storage function; t

  passing in the wasm itself rather than referencing a previously stored contract is  

For example, 

   passing in wasm itself
  see basic description of deploy.



### Store (~~workflow~~)

1. store a contract by sending the wasm using `--session`

 1. call the contract by name/address 

 `--session-name` / 
 `--session-hash` 

For Example, 

 see `count-define` [Deploy](https://github.com/CasperLabs/CasperLabs/tree/dev/execution-engine/contracts/examples/counter-define)

Explanation: 

