## DEVNET

## Quick Start

## Instructions

##### Step 1: Creating an account

* Go to [clarity.casperlabs.io](https://clarity.casperlabs.io/)
 [Sign-in] and complete the new user process
 
* Navigate to [Accounts](https://clarity.casperlabs.io/#/accounts) and click [Create Account]

    Provide a name for your account and click [Save]; multiple key files will be written to disk. You will need these key files to interact with the network, so give some consideration to where you save them.

* Navigate to [Faucet](https://clarity.casperlabs.io/#/faucet), select your new account, and click [Request Tokens]
 The request will appear in the "Recent Faucet Requests" table; wait for the green check mark to appear in the "Status" column 
 
  **Note:** that a "token" obtained on devnet has no monetary value whatsoever

##### Step 2:  Checking the balance of an account

* [Install the CasperLabs client](INSTALL.md)

* Use the `balance` sub-command (see example below)
  * `--address` is the hex-encoded public key of the account to query
  * `--block-hash` the hex-encoded hash of the block where the balance should be queried

```
casperlabs-client \
        --host deploy.casperlabs.io \
        balance \
        --address b9ae114d6093646ed4be6f7fe1f7e5533a5d52a3351f3f18030ea82b3d915d6b \
        --block-hash ef6d4c66a29d833de462fbb7fd35227cbc3849b36872940c852727f668d6993f
```

##### Step 3: Deploying code

* [Install the CasperLabs client](INSTALL.md)
* [Compile a contract written in rust](CONTRACTS.md)
* Use the CasperLabs client `deploy` sub-command (example below)
  - `--session` is the path to the compiled contract
  - `--private-key` is the path to the private key file downloaded from [clarity.casperlabs.io](https://clarity.casperlabs.io/) during account creation
```
casperlabs-client \
        --host deploy.casperlabs.io \
        deploy \
        --session <path-to-wasm> \
        --private-key account.private.key
```

* You can also monitor the outcome of deploys using the casperlabs-client:
```
casperlabs-client\ 
        --host deploy.casperlabs.io \ 
        --port 40401 show-deploy <deploy-hash>
```


##### Step 4: Adding Parameters for Time to Live and Deploy Dependencies

* [Install the CasperLabs client](INSTALL.md)
* [Compile a contract written in rust](CONTRACTS.md)
* Use the CasperLabs client `deploy` sub-command (example below)

**The `time_to_live` parameter:** can be included in the deploy to set a time frame limiting how long a deploy is effective before it is no longer valid.

  * Use the CasperLabs client `deploy` sub-command (example below)
  
    * `--ttl-millis` add the argument set Time to live, Time (in milliseconds) that the deploy will remain valid for. 

```
casperlabs-client\
    --host deploy.casperlabs.io \
        deploy \
    --ttl-millis  <arg>
```
       

**The `dependencies` parameter:** provides for listing the deploy hashes that must be executed before the present one explicitly enforcing an ordering to deploys.

* Use the `deploy` sub-command (see example below)

  * `--dependencies` list deploy hashes (base16 encoded) which must be executed before this deploy.
  
```
casperlabs-client\
    --host deploy.casperlabs.io \
        deploy \
    --dependencies  <arg>...
```

See `casperlabs-client deploy --help`

##### Step 5: Bonding

Follow instructions in [NODE.md](NODE.md) for connecting to the CasperLabs network

Once bonded, you can use the CasperLabs client with your local node to deploy code and propose blocks on the devnet
  - See [CONTRACTS.md](CONTRACTS.md) for details
```
casperlabs-client \
        --host localhost \
        deploy \
        --session <path-to-wasm> \
        --private-key <path-to-account-private-key>

casperlabs-client \
        --host localhost \
        propose
```

##### Step 6: Unbonding

* Follow instructions in [NODE.md](NODE.md) for stopping a bonded validator

## Notes

* This quick start gives the simplest set of instructions for getting started on the CasperLabs devnet. More advanced users may wish to take other approaches to some of the steps listed above.
