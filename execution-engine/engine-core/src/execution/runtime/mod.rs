mod args;
mod externals;

use std::{
    cmp,
    collections::{BTreeMap, HashMap, HashSet},
    convert::TryFrom,
    iter::IntoIterator,
};

use itertools::Itertools;
use parity_wasm::elements::Module;
use wasmi::{ImportsBuilder, MemoryRef, ModuleInstance, ModuleRef, Trap, TrapKind};

use contract_ffi::{
    args_parser::ArgsParser,
    bytesrepr::{deserialize, ToBytes},
    contract_api::{
        system::{TransferResult, TransferredTo},
        Error as ApiError,
    },
    key::Key,
    system_contracts::{self, mint, SystemContract},
    uref::{AccessRights, URef},
    value::{
        account::{ActionType, PublicKey, PurseId, Weight, PUBLIC_KEY_SERIALIZED_LENGTH},
        Account, ProtocolVersion, Value, U512,
    },
};
use engine_shared::gas::Gas;
use engine_storage::global_state::StateReader;

use super::{Error, MINT_NAME, POS_NAME};
use crate::{
    engine_state::system_contract_cache::SystemContractCache,
    resolvers::{create_module_resolver, memory_resolver::MemoryResolver},
    runtime_context::RuntimeContext,
    Address,
};

pub struct Runtime<'a, R> {
    system_contract_cache: SystemContractCache,
    memory: MemoryRef,
    module: Module,
    result: Vec<u8>,
    host_buf: Option<Vec<u8>>,
    context: RuntimeContext<'a, R>,
}

/// Rename function called `name` in the `module` to `call`.
/// wasmi's entrypoint for a contracts is a function called `call`,
/// so we have to rename function before storing it in the GlobalState.
pub fn rename_export_to_call(module: &mut Module, name: String) {
    let main_export = module
        .export_section_mut()
        .unwrap()
        .entries_mut()
        .iter_mut()
        .find(|e| e.field() == name)
        .unwrap()
        .field_mut();
    main_export.clear();
    main_export.push_str("call");
}

pub fn instance_and_memory(
    parity_module: Module,
    protocol_version: ProtocolVersion,
) -> Result<(ModuleRef, MemoryRef), Error> {
    let module = wasmi::Module::from_parity_wasm_module(parity_module)?;
    let resolver = create_module_resolver(protocol_version)?;
    let mut imports = ImportsBuilder::new();
    imports.push_resolver("env", &resolver);
    let instance = ModuleInstance::new(&module, &imports)?.assert_no_start();

    let memory = resolver.memory_ref()?;
    Ok((instance, memory))
}

/// Turns `key` into a `([u8; 32], AccessRights)` tuple.
/// Returns None if `key` is not `Key::URef` as it wouldn't have `AccessRights`
/// associated with it. Helper function for creating `named_keys` associating
/// addresses and corresponding `AccessRights`.
pub fn key_to_tuple(key: Key) -> Option<([u8; 32], Option<AccessRights>)> {
    match key {
        Key::URef(uref) => Some((uref.addr(), uref.access_rights())),
        Key::Account(_) => None,
        Key::Hash(_) => None,
        Key::Local { .. } => None,
    }
}

/// Groups a collection of urefs by their addresses and accumulates access
/// rights per key
pub fn extract_access_rights_from_urefs<I: IntoIterator<Item = URef>>(
    input: I,
) -> HashMap<Address, HashSet<AccessRights>> {
    input
        .into_iter()
        .map(|uref: URef| (uref.addr(), uref.access_rights()))
        .group_by(|(key, _)| *key)
        .into_iter()
        .map(|(key, group)| {
            (
                key,
                group
                    .filter_map(|(_, x)| x)
                    .collect::<HashSet<AccessRights>>(),
            )
        })
        .collect()
}

/// Groups a collection of keys by their address and accumulates access rights
/// per key.
pub fn extract_access_rights_from_keys<I: IntoIterator<Item = Key>>(
    input: I,
) -> HashMap<Address, HashSet<AccessRights>> {
    input
        .into_iter()
        .map(key_to_tuple)
        .flatten()
        .group_by(|(key, _)| *key)
        .into_iter()
        .map(|(key, group)| {
            (
                key,
                group
                    .filter_map(|(_, x)| x)
                    .collect::<HashSet<AccessRights>>(),
            )
        })
        .collect()
}

fn sub_call<R: StateReader<Key, Value>>(
    parity_module: Module,
    args: Vec<Vec<u8>>,
    named_keys: &mut BTreeMap<String, Key>,
    key: Key,
    current_runtime: &mut Runtime<R>,
    // Unforgable references passed across the call boundary from caller to callee
    //(necessary if the contract takes a uref argument).
    extra_urefs: Vec<Key>,
    protocol_version: ProtocolVersion,
) -> Result<Vec<u8>, Error>
where
    R::Error: Into<Error>,
{
    let (instance, memory) = instance_and_memory(parity_module.clone(), protocol_version)?;

    let access_rights = {
        let mut keys: Vec<Key> = named_keys.values().cloned().collect();
        keys.extend(extra_urefs);
        keys.push(current_runtime.get_mint_contract_uref().into());
        keys.push(current_runtime.get_pos_contract_uref().into());
        extract_access_rights_from_keys(keys)
    };

    let system_contract_cache = SystemContractCache::clone(&current_runtime.system_contract_cache);

    let mut runtime = Runtime {
        system_contract_cache,
        memory,
        module: parity_module,
        result: Vec::new(),
        host_buf: None,
        context: RuntimeContext::new(
            current_runtime.context.state(),
            named_keys,
            access_rights,
            args,
            current_runtime.context.authorization_keys().clone(),
            &current_runtime.context.account(),
            key,
            current_runtime.context.get_blocktime(),
            current_runtime.context.get_deployhash(),
            current_runtime.context.gas_limit(),
            current_runtime.context.gas_counter(),
            current_runtime.context.fn_store_id(),
            current_runtime.context.address_generator(),
            protocol_version,
            current_runtime.context.correlation_id(),
            current_runtime.context.phase(),
            current_runtime.context.protocol_data(),
        ),
    };

    let result = instance.invoke_export("call", &[], &mut runtime);

    match result {
        Ok(_) => Ok(runtime.result),
        Err(e) => {
            if let Some(host_error) = e.as_host_error() {
                // If the "error" was in fact a trap caused by calling `ret` then
                // this is normal operation and we should return the value captured
                // in the Runtime result field.
                let downcasted_error = host_error.downcast_ref::<Error>().unwrap();
                match downcasted_error {
                    Error::Ret(ref ret_urefs) => {
                        //insert extra urefs returned from call
                        let ret_urefs_map: HashMap<Address, HashSet<AccessRights>> =
                            extract_access_rights_from_urefs(ret_urefs.clone());
                        current_runtime.context.access_rights_extend(ret_urefs_map);
                        return Ok(runtime.result);
                    }
                    Error::Revert(status) => {
                        // Propagate revert as revert, instead of passing it as
                        // InterpreterError.
                        return Err(Error::Revert(*status));
                    }
                    Error::InvalidContext => {
                        // TODO: https://casperlabs.atlassian.net/browse/EE-771
                        return Err(Error::InvalidContext);
                    }
                    _ => {}
                }
            }
            Err(Error::Interpreter(e))
        }
    }
}

impl<'a, R: StateReader<Key, Value>> Runtime<'a, R>
where
    R::Error: Into<Error>,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        system_contract_cache: SystemContractCache,
        memory: MemoryRef,
        module: Module,
        context: RuntimeContext<'a, R>,
    ) -> Self {
        Runtime {
            system_contract_cache,
            memory,
            module,
            result: Vec::new(),
            host_buf: None,
            context,
        }
    }

    pub fn result(&self) -> &[u8] {
        self.result.as_slice()
    }

    pub fn context(&self) -> &RuntimeContext<'a, R> {
        &self.context
    }

    /// Charge specified amount of gas
    ///
    /// Returns false if gas limit exceeded and true if not.
    /// Intuition about the return value sense is to aswer the question 'are we
    /// allowed to continue?'
    fn charge_gas(&mut self, amount: Gas) -> bool {
        let prev = self.context.gas_counter();
        match prev.checked_add(amount) {
            // gas charge overflow protection
            None => false,
            Some(val) if val > self.context.gas_limit() => false,
            Some(val) => {
                self.context.set_gas_counter(val);
                true
            }
        }
    }

    fn gas(&mut self, amount: Gas) -> Result<(), Trap> {
        if self.charge_gas(amount) {
            Ok(())
        } else {
            Err(Error::GasLimit.into())
        }
    }

    fn bytes_from_mem(&self, ptr: u32, size: usize) -> Result<Vec<u8>, Error> {
        self.memory.get(ptr, size).map_err(Into::into)
    }

    /// Reads key (defined as `key_ptr` and `key_size` tuple) from Wasm memory.
    fn key_from_mem(&mut self, key_ptr: u32, key_size: u32) -> Result<Key, Error> {
        let bytes = self.bytes_from_mem(key_ptr, key_size as usize)?;
        deserialize(&bytes).map_err(Into::into)
    }

    /// Reads value (defined as `value_ptr` and `value_size` tuple) from Wasm
    /// memory.
    fn value_from_mem(&mut self, value_ptr: u32, value_size: u32) -> Result<Value, Error> {
        let bytes = self.bytes_from_mem(value_ptr, value_size as usize)?;
        deserialize(&bytes).map_err(Into::into)
    }

    fn string_from_mem(&self, ptr: u32, size: u32) -> Result<String, Trap> {
        let bytes = self.bytes_from_mem(ptr, size as usize)?;
        deserialize(&bytes).map_err(|e| Error::BytesRepr(e).into())
    }

    fn get_function_by_name(&mut self, name_ptr: u32, name_size: u32) -> Result<Vec<u8>, Trap> {
        let name = self.string_from_mem(name_ptr, name_size)?;

        let has_name: bool = self
            .module
            .export_section()
            .and_then(|es| es.entries().iter().find(|e| e.field() == name))
            .is_some();

        if has_name {
            let mut module = self.module.clone();
            // We only want the function exported under `name` to be callable;
            //`optimize` removes all code that is not reachable from the exports
            // listed in the second argument.
            pwasm_utils::optimize(&mut module, vec![&name]).unwrap();
            rename_export_to_call(&mut module, name);

            parity_wasm::serialize(module).map_err(|e| Error::ParityWasm(e).into())
        } else {
            Err(Error::FunctionNotFound(name).into())
        }
    }

    pub fn value_is_valid(&mut self, value_ptr: u32, value_size: u32) -> Result<bool, Trap> {
        let value = self.value_from_mem(value_ptr, value_size)?;

        Ok(self.context.validate_value(&value).is_ok())
    }

    /// Load the i-th argument invoked as part of a `sub_call` into
    /// the runtime buffer so that a subsequent `get_arg` can return it
    /// to the caller.
    pub fn load_arg(&mut self, i: usize) -> isize {
        match self.context.args().get(i) {
            Some(arg) => {
                // NOTE: This will be changed once #1426 (EE-801) will be merged in
                self.host_buf = Some(arg.clone());
                arg.len() as isize
            }
            None => {
                self.host_buf = None;
                -1
            }
        }
    }

    pub fn get_arg_size(
        &mut self,
        index: usize,
        size_ptr: u32,
    ) -> Result<Result<(), ApiError>, Trap> {
        let arg_size = match self.context.args().get(index) {
            Some(arg) if arg.len() > u32::max_value() as usize => {
                return Ok(Err(ApiError::OutOfMemoryError))
            }
            None => return Ok(Err(ApiError::MissingArgument)),
            Some(arg) => arg.len() as u32,
        };

        let arg_size_bytes = arg_size.to_le_bytes(); // wasm is LE

        if let Err(e) = self.memory.set(size_ptr, &arg_size_bytes) {
            return Err(Error::Interpreter(e).into());
        }

        Ok(Ok(()))
    }

    pub fn get_arg(
        &mut self,
        index: usize,
        output_ptr: u32,
        output_size: usize,
    ) -> Result<Result<(), ApiError>, Trap> {
        let arg = match self.context.args().get(index) {
            Some(arg) => arg,
            None => return Ok(Err(ApiError::MissingArgument)),
        };

        if arg.len() > output_size {
            return Ok(Err(ApiError::OutOfMemoryError));
        }

        if let Err(e) = self.memory.set(output_ptr, &arg[..output_size]) {
            return Err(Error::Interpreter(e).into());
        }

        Ok(Ok(()))
    }

    /// Load the uref known by the given name into the Wasm memory
    pub fn load_key(
        &mut self,
        name_ptr: u32,
        name_size: u32,
        output_ptr: u32,
        output_size: usize,
        bytes_written_ptr: u32,
    ) -> Result<Result<(), ApiError>, Trap> {
        let name = self.string_from_mem(name_ptr, name_size)?;

        // Get a key and serialize it
        let key = match self.context.named_keys_get(&name) {
            Some(key) => key,
            None => return Ok(Err(ApiError::MissingKey)),
        };

        let key_bytes = match key.to_bytes() {
            Ok(bytes) => bytes,
            Err(error) => return Ok(Err(error.into())),
        };

        // `output_size` has to be greater or equal to the actual length of serialized Key bytes
        if output_size < key_bytes.len() {
            return Ok(Err(ApiError::BufferTooSmall));
        }

        // Set serialized Key bytes into the output buffer
        if let Err(error) = self.memory.set(output_ptr, &key_bytes) {
            return Err(Error::Interpreter(error).into());
        }

        // For all practical purposes following cast is assumed to be safe
        let bytes_size = key_bytes.len() as u32;
        let size_bytes = bytes_size.to_le_bytes(); // wasm is LE
        if let Err(error) = self.memory.set(bytes_written_ptr, &size_bytes) {
            return Err(Error::Interpreter(error).into());
        }

        Ok(Ok(()))
    }

    pub fn has_key(&mut self, name_ptr: u32, name_size: u32) -> Result<i32, Trap> {
        let name = self.string_from_mem(name_ptr, name_size)?;
        if self.context.named_keys_contains_key(&name) {
            Ok(0)
        } else {
            Ok(1)
        }
    }

    pub fn put_key(
        &mut self,
        name_ptr: u32,
        name_size: u32,
        key_ptr: u32,
        key_size: u32,
    ) -> Result<(), Trap> {
        let name = self.string_from_mem(name_ptr, name_size)?;
        let key = self.key_from_mem(key_ptr, key_size)?;
        self.context.put_key(name, key).map_err(Into::into)
    }

    fn remove_key(&mut self, name_ptr: u32, name_size: u32) -> Result<(), Trap> {
        let name = self.string_from_mem(name_ptr, name_size)?;
        self.context.remove_key(&name)?;
        Ok(())
    }

    /// Writes runtime context's account main purse to [dest_ptr] in the Wasm memory.
    fn get_main_purse(&mut self, dest_ptr: u32) -> Result<(), Trap> {
        let purse_id = self.context.get_main_purse()?;
        let purse_id_bytes = purse_id.to_bytes().map_err(Error::BytesRepr)?;
        self.memory
            .set(dest_ptr, &purse_id_bytes)
            .map_err(|e| Error::Interpreter(e).into())
    }

    /// Writes caller (deploy) account public key to [dest_ptr] in the Wasm
    /// memory.
    fn get_caller(&mut self, dest_ptr: u32) -> Result<(), Trap> {
        let key = self.context.get_caller();
        let bytes = key.to_bytes().map_err(Error::BytesRepr)?;
        self.memory
            .set(dest_ptr, &bytes)
            .map_err(|e| Error::Interpreter(e).into())
    }

    /// Writes runtime context's phase to [dest_ptr] in the Wasm memory.
    fn get_phase(&mut self, dest_ptr: u32) -> Result<(), Trap> {
        let phase = self.context.phase();
        let bytes = phase.to_bytes().map_err(Error::BytesRepr)?;
        self.memory
            .set(dest_ptr, &bytes)
            .map_err(|e| Error::Interpreter(e).into())
    }

    /// Writes current blocktime to [dest_ptr] in Wasm memory.
    fn get_blocktime(&self, dest_ptr: u32) -> Result<(), Trap> {
        let blocktime = self
            .context
            .get_blocktime()
            .to_bytes()
            .map_err(Error::BytesRepr)?;
        self.memory
            .set(dest_ptr, &blocktime)
            .map_err(|e| Error::Interpreter(e).into())
    }

    /// Return a some bytes from the memory and terminate the current
    /// `sub_call`. Note that the return type is `Trap`, indicating that
    /// this function will always kill the current Wasm instance.
    pub fn ret(
        &mut self,
        value_ptr: u32,
        value_size: usize,
        extra_urefs_ptr: u32,
        extra_urefs_size: usize,
    ) -> Trap {
        let mem_get = self
            .memory
            .get(value_ptr, value_size)
            .map_err(Error::Interpreter)
            .and_then(|x| {
                let urefs_bytes = self.bytes_from_mem(extra_urefs_ptr, extra_urefs_size)?;
                let urefs = self.context.deserialize_urefs(&urefs_bytes)?;
                Ok((x, urefs))
            });
        match mem_get {
            Ok((buf, urefs)) => {
                // Set the result field in the runtime and return
                // the proper element of the `Error` enum indicating
                // that the reason for exiting the module was a call to ret.
                self.result = buf;
                Error::Ret(urefs).into()
            }
            Err(e) => e.into(),
        }
    }

    /// Calls contract living under a `key`, with supplied `args` and extra
    /// `urefs`.
    pub fn call_contract(
        &mut self,
        key: Key,
        args_bytes: Vec<u8>,
        urefs_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, Error> {
        let contract = match self.context.read_gs(&key)? {
            Some(Value::Contract(contract)) => contract,
            Some(_) => {
                return Err(Error::FunctionNotFound(format!(
                    "Value at {:?} is not a contract",
                    key
                )))
            }
            None => return Err(Error::KeyNotFound(key)),
        };

        // Check for major version compatibility before calling
        let contract_version = contract.protocol_version();
        let current_version = self.context.protocol_version();
        if !contract_version.is_compatible_with(&current_version) {
            return Err(Error::IncompatibleProtocolMajorVersion {
                actual: current_version.value().major,
                expected: contract_version.value().major,
            });
        }

        let args: Vec<Vec<u8>> = deserialize(&args_bytes)?;

        let maybe_module = match key {
            Key::URef(uref) => self.system_contract_cache.get(&uref),
            _ => None,
        };

        let module = match maybe_module {
            Some(module) => module,
            None => parity_wasm::deserialize_buffer(contract.bytes())?,
        };

        let extra_urefs = self.context.deserialize_keys(&urefs_bytes)?;

        let mut refs = contract.take_named_keys();

        let result = sub_call(
            module,
            args,
            &mut refs,
            key,
            self,
            extra_urefs,
            contract_version,
        )?;
        Ok(result)
    }

    pub fn call_contract_host_buf(
        &mut self,
        key: Key,
        args_bytes: Vec<u8>,
        urefs_bytes: Vec<u8>,
        result_size_ptr: u32,
    ) -> Result<Result<(), ApiError>, Error> {
        if !self.can_write_to_host_buf() {
            // Exit early if the host buffer is already occupied
            return Ok(Err(ApiError::HostBufferFull));
        }

        let result_bytes = self.call_contract(key, args_bytes, urefs_bytes)?;
        let result_size = result_bytes.len() as u32; // considered to be safe

        match self.write_host_buf(result_bytes) {
            Ok(_) => {}
            Err(error) => return Ok(Err(error)),
        }

        let result_size_bytes = result_size.to_le_bytes(); // wasm is LE
        if let Err(error) = self.memory.set(result_size_ptr, &result_size_bytes) {
            return Err(Error::Interpreter(error));
        }

        Ok(Ok(()))
    }

    fn load_named_keys(
        &mut self,
        total_keys_ptr: u32,
        result_size_ptr: u32,
    ) -> Result<Result<(), ApiError>, Trap> {
        if !self.can_write_to_host_buf() {
            // Exit early if the host buffer is already occupied
            return Ok(Err(ApiError::HostBufferFull));
        }

        let named_keys = self.context.named_keys();

        let total_keys = named_keys.len() as u32;
        let total_keys_bytes = total_keys.to_le_bytes();
        if let Err(error) = self.memory.set(total_keys_ptr, &total_keys_bytes) {
            return Err(Error::Interpreter(error).into());
        }

        if total_keys == 0 {
            // No need to do anything else, we leave host buffer empty.
            self.host_buf = None;
            return Ok(Ok(()));
        }

        let bytes = match named_keys.to_bytes() {
            Ok(bytes) => bytes,
            Err(error) => return Ok(Err(error.into())),
        };

        let length = bytes.len() as u32;
        if let Err(error) = self.write_host_buf(bytes) {
            return Ok(Err(error));
        }

        let length_bytes = length.to_le_bytes();
        if let Err(error) = self.memory.set(result_size_ptr, &length_bytes) {
            return Err(Error::Interpreter(error).into());
        }

        Ok(Ok(()))
    }

    pub fn store_function(
        &mut self,
        fn_bytes: Vec<u8>,
        named_keys: BTreeMap<String, Key>,
    ) -> Result<[u8; 32], Error> {
        let contract = contract_ffi::value::contract::Contract::new(
            fn_bytes,
            named_keys,
            self.context.protocol_version(),
        );
        let contract_addr = self.context.store_function(contract.into())?;
        Ok(contract_addr)
    }

    /// Tries to store a function, represented as bytes from the Wasm memory,
    /// into the GlobalState and writes back a function's hash at `hash_ptr`
    /// in the Wasm memory.
    pub fn store_function_at_hash(
        &mut self,
        fn_bytes: Vec<u8>,
        named_keys: BTreeMap<String, Key>,
    ) -> Result<[u8; 32], Error> {
        let contract = contract_ffi::value::contract::Contract::new(
            fn_bytes,
            named_keys,
            self.context.protocol_version(),
        );
        let new_hash = self.context.store_function_at_hash(contract.into())?;
        Ok(new_hash)
    }

    /// Writes function address (`hash_bytes`) into the Wasm memory (at
    /// `dest_ptr` pointer).
    fn function_address(&mut self, hash_bytes: [u8; 32], dest_ptr: u32) -> Result<(), Trap> {
        self.memory
            .set(dest_ptr, &hash_bytes)
            .map_err(|e| Error::Interpreter(e).into())
    }

    /// Generates new unforgable reference and adds it to the context's
    /// access_rights set.
    pub fn new_uref(&mut self, key_ptr: u32, value_ptr: u32, value_size: u32) -> Result<(), Trap> {
        let value = self.value_from_mem(value_ptr, value_size)?; // read initial value from memory
        let key = self.context.new_uref(value)?;
        self.memory
            .set(key_ptr, &key.to_bytes().map_err(Error::BytesRepr)?)
            .map_err(|e| Error::Interpreter(e).into())
    }

    /// Writes `value` under `key` in GlobalState.
    pub fn write(
        &mut self,
        key_ptr: u32,
        key_size: u32,
        value_ptr: u32,
        value_size: u32,
    ) -> Result<(), Trap> {
        let key = self.key_from_mem(key_ptr, key_size)?;
        let value = self.value_from_mem(value_ptr, value_size)?;
        self.context.write_gs(key, value).map_err(Into::into)
    }

    /// Writes `value` under a key derived from `key` in the "local cluster" of
    /// GlobalState
    pub fn write_local(
        &mut self,
        key_ptr: u32,
        key_size: u32,
        value_ptr: u32,
        value_size: u32,
    ) -> Result<(), Trap> {
        let key_bytes = self.bytes_from_mem(key_ptr, key_size as usize)?;
        let value = self.value_from_mem(value_ptr, value_size)?;
        self.context.write_ls(&key_bytes, value).map_err(Into::into)
    }

    /// Adds `value` to the cell that `key` points at.
    pub fn add(
        &mut self,
        key_ptr: u32,
        key_size: u32,
        value_ptr: u32,
        value_size: u32,
    ) -> Result<(), Trap> {
        let key = self.key_from_mem(key_ptr, key_size)?;
        let value = self.value_from_mem(value_ptr, value_size)?;
        self.context.add_gs(key, value).map_err(Into::into)
    }

    /// Reads value from the GS living under key specified by `key_ptr` and
    /// `key_size`. Wasm and host communicate through memory that Wasm
    /// module exports. If contract wants to pass data to the host, it has
    /// to tell it [the host] where this data lives in the exported memory
    /// (pass its pointer and length).
    pub fn read(
        &mut self,
        key_ptr: u32,
        key_size: u32,
        output_size_ptr: u32,
    ) -> Result<Result<(), ApiError>, Trap> {
        if !self.can_write_to_host_buf() {
            // Exit early if the host buffer is already occupied
            return Ok(Err(ApiError::HostBufferFull));
        }

        let key = self.key_from_mem(key_ptr, key_size)?;
        let value = match self.context.read_gs(&key)? {
            Some(value) => value,
            None => return Ok(Err(ApiError::ValueNotFound)),
        };

        let value_bytes = match value.to_bytes() {
            Ok(bytes) => bytes,
            Err(error) => return Ok(Err(error.into())),
        };

        let value_size = value_bytes.len() as u32;
        if let Err(error) = self.write_host_buf(value_bytes) {
            return Ok(Err(error));
        }

        let value_bytes = value_size.to_le_bytes(); // wasm is LE
        if let Err(error) = self.memory.set(output_size_ptr, &value_bytes) {
            return Err(Error::Interpreter(error).into());
        }

        Ok(Ok(()))
    }

    /// Similar to `read`, this function is for reading from the "local cluster"
    /// of global state
    pub fn read_local(
        &mut self,
        key_ptr: u32,
        key_size: u32,
        output_size_ptr: u32,
    ) -> Result<Result<(), ApiError>, Trap> {
        if !self.can_write_to_host_buf() {
            // Exit early if the host buffer is already occupied
            return Ok(Err(ApiError::HostBufferFull));
        }

        let key_bytes = self.bytes_from_mem(key_ptr, key_size as usize)?;

        let value = match self.context.read_ls(&key_bytes)? {
            Some(value) => value,
            None => return Ok(Err(ApiError::ValueNotFound)),
        };

        let value_bytes = match value.to_bytes() {
            Ok(bytes) => bytes,
            Err(error) => return Ok(Err(error.into())),
        };

        let value_size = value_bytes.len() as u32;
        if let Err(error) = self.write_host_buf(value_bytes) {
            return Ok(Err(error));
        }

        let value_bytes = value_size.to_le_bytes(); // wasm is LE
        if let Err(error) = self.memory.set(output_size_ptr, &value_bytes) {
            return Err(Error::Interpreter(error).into());
        }

        Ok(Ok(()))
    }

    /// Reverts contract execution with a status specified.
    pub fn revert(&mut self, status: u32) -> Trap {
        Error::Revert(status).into()
    }

    pub fn take_context(self) -> RuntimeContext<'a, R> {
        self.context
    }

    fn add_associated_key(&mut self, public_key_ptr: u32, weight_value: u8) -> Result<i32, Trap> {
        let public_key = {
            // Public key as serialized bytes
            let source_serialized =
                self.bytes_from_mem(public_key_ptr, PUBLIC_KEY_SERIALIZED_LENGTH)?;
            // Public key deserialized
            let source: PublicKey = deserialize(&source_serialized).map_err(Error::BytesRepr)?;
            source
        };
        let weight = Weight::new(weight_value);

        match self.context.add_associated_key(public_key, weight) {
            Ok(_) => Ok(0),
            // This relies on the fact that `AddKeyFailure` is represented as
            // i32 and first variant start with number `1`, so all other variants
            // are greater than the first one, so it's safe to assume `0` is success,
            // and any error is greater than 0.
            Err(Error::AddKeyFailure(e)) => Ok(e as i32),
            // Any other variant just pass as `Trap`
            Err(e) => Err(e.into()),
        }
    }

    fn remove_associated_key(&mut self, public_key_ptr: u32) -> Result<i32, Trap> {
        let public_key = {
            // Public key as serialized bytes
            let source_serialized =
                self.bytes_from_mem(public_key_ptr, PUBLIC_KEY_SERIALIZED_LENGTH)?;
            // Public key deserialized
            let source: PublicKey = deserialize(&source_serialized).map_err(Error::BytesRepr)?;
            source
        };
        match self.context.remove_associated_key(public_key) {
            Ok(_) => Ok(0),
            Err(Error::RemoveKeyFailure(e)) => Ok(e as i32),
            Err(e) => Err(e.into()),
        }
    }

    fn update_associated_key(
        &mut self,
        public_key_ptr: u32,
        weight_value: u8,
    ) -> Result<i32, Trap> {
        let public_key = {
            // Public key as serialized bytes
            let source_serialized =
                self.bytes_from_mem(public_key_ptr, PUBLIC_KEY_SERIALIZED_LENGTH)?;
            // Public key deserialized
            let source: PublicKey = deserialize(&source_serialized).map_err(Error::BytesRepr)?;
            source
        };
        let weight = Weight::new(weight_value);

        match self.context.update_associated_key(public_key, weight) {
            Ok(_) => Ok(0),
            // This relies on the fact that `UpdateKeyFailure` is represented as
            // i32 and first variant start with number `1`, so all other variants
            // are greater than the first one, so it's safe to assume `0` is success,
            // and any error is greater than 0.
            Err(Error::UpdateKeyFailure(e)) => Ok(e as i32),
            // Any other variant just pass as `Trap`
            Err(e) => Err(e.into()),
        }
    }

    fn set_action_threshold(
        &mut self,
        action_type_value: u32,
        threshold_value: u8,
    ) -> Result<i32, Trap> {
        match ActionType::try_from(action_type_value) {
            Ok(action_type) => {
                let threshold = Weight::new(threshold_value);
                match self.context.set_action_threshold(action_type, threshold) {
                    Ok(_) => Ok(0),
                    Err(Error::SetThresholdFailure(e)) => Ok(e as i32),
                    Err(e) => Err(e.into()),
                }
            }
            Err(_) => Err(Trap::new(TrapKind::Unreachable)),
        }
    }

    /// Looks up the public mint contract key in the context's protocol data.
    ///
    /// Returned URef is already attenuated depending on the calling account.
    pub fn get_mint_contract_uref(&mut self) -> URef {
        let mint = self.context.protocol_data().mint();
        self.context.attenuate_uref(mint)
    }

    /// Looks up the public PoS contract key in the context's protocol data
    ///
    /// Returned URef is already attenuated depending on the calling account.
    pub fn get_pos_contract_uref(&mut self) -> URef {
        let pos = self.context.protocol_data().proof_of_stake();
        self.context.attenuate_uref(pos)
    }

    /// Calls the "create" method on the mint contract at the given mint
    /// contract key
    fn mint_create(&mut self, mint_contract_key: Key) -> Result<PurseId, Error> {
        let args_bytes = {
            let args = ("create",);
            ArgsParser::parse(&args).and_then(|args| args.to_bytes())?
        };

        let urefs_bytes = Vec::<Key>::new().to_bytes()?;

        let result_bytes = self.call_contract(mint_contract_key, args_bytes, urefs_bytes)?;

        let result: URef = deserialize(&result_bytes)?;

        Ok(PurseId::new(result))
    }

    fn create_purse(&mut self) -> Result<PurseId, Error> {
        let mint_contract_key = self.get_mint_contract_uref().into();
        self.mint_create(mint_contract_key)
    }

    /// Calls the "transfer" method on the mint contract at the given mint
    /// contract key
    fn mint_transfer(
        &mut self,
        mint_contract_key: Key,
        source: PurseId,
        target: PurseId,
        amount: U512,
    ) -> Result<(), Error> {
        let source_value: URef = source.value();
        let target_value: URef = target.value();

        let args_bytes = {
            let args = ("transfer", source_value, target_value, amount);
            ArgsParser::parse(&args).and_then(|args| args.to_bytes())?
        };

        let urefs_bytes = vec![Key::URef(source_value), Key::URef(target_value)].to_bytes()?;

        let result_bytes = self.call_contract(mint_contract_key, args_bytes, urefs_bytes)?;

        // This will deserialize `host_buf` into the Result type which carries
        // mint contract error.
        let result: Result<(), mint::Error> = deserialize(&result_bytes)?;
        // Wraps mint error into a more general error type through an aggregate
        // system contracts Error.
        Ok(result.map_err(system_contracts::Error::from)?)
    }

    /// Creates a new account at a given public key, transferring a given amount
    /// of motes from the given source purse to the new account's purse.
    fn transfer_to_new_account(
        &mut self,
        source: PurseId,
        target: PublicKey,
        amount: U512,
    ) -> Result<TransferResult, Error> {
        let mint_contract_key = self.get_mint_contract_uref().into();

        let target_addr = target.value();
        let target_key = Key::Account(target_addr);

        // A precondition check that verifies that the transfer can be done
        // as the source purse has enough funds to cover the transfer.
        if amount > self.get_balance(source)?.unwrap_or_default() {
            return Ok(Err(ApiError::Transfer));
        }

        let target_purse_id = self.mint_create(mint_contract_key)?;

        if source == target_purse_id {
            return Ok(Err(ApiError::Transfer));
        }

        match self.mint_transfer(mint_contract_key, source, target_purse_id, amount) {
            Ok(_) => {
                // After merging in EE-704 system contracts lookup internally uses protocol data and
                // this is used for backwards compatibility with explorer to query mint/pos urefs.
                let named_keys = vec![
                    (
                        String::from(MINT_NAME),
                        Key::from(self.get_mint_contract_uref()),
                    ),
                    (
                        String::from(POS_NAME),
                        Key::from(self.get_pos_contract_uref()),
                    ),
                ]
                .into_iter()
                .map(|(name, key)| {
                    if let Some(uref) = key.as_uref() {
                        (name, Key::URef(URef::new(uref.addr(), AccessRights::READ)))
                    } else {
                        (name, key)
                    }
                })
                .collect();
                let account = Account::create(target_addr, named_keys, target_purse_id);
                self.context.write_account(target_key, account)?;
                Ok(Ok(TransferredTo::NewAccount))
            }
            Err(_) => Ok(Err(ApiError::Transfer)),
        }
    }

    /// Transferring a given amount of motes from the given source purse to the
    /// new account's purse. Requires that the [`PurseId`]s have already
    /// been created by the mint contract (or are the genesis account's).
    fn transfer_to_existing_account(
        &mut self,
        source: PurseId,
        target: PurseId,
        amount: U512,
    ) -> Result<TransferResult, Error> {
        let mint_contract_key = self.get_mint_contract_uref().into();

        // This appears to be a load-bearing use of `RuntimeContext::insert_uref`.
        self.context.insert_uref(target.value());

        match self.mint_transfer(mint_contract_key, source, target, amount) {
            Ok(_) => Ok(Ok(TransferredTo::ExistingAccount)),
            Err(_) => Ok(Err(ApiError::Transfer)),
        }
    }

    /// Transfers `amount` of motes from default purse of the account to
    /// `target` account. If that account does not exist, creates one.
    fn transfer_to_account(
        &mut self,
        target: PublicKey,
        amount: U512,
    ) -> Result<TransferResult, Error> {
        let source = self.context.get_main_purse()?;
        self.transfer_from_purse_to_account(source, target, amount)
    }

    /// Transfers `amount` of motes from `source` purse to `target` account.
    /// If that account does not exist, creates one.
    fn transfer_from_purse_to_account(
        &mut self,
        source: PurseId,
        target: PublicKey,
        amount: U512,
    ) -> Result<TransferResult, Error> {
        let target_key = Key::Account(target.value());
        // Look up the account at the given public key's address
        match self.context.read_account(&target_key)? {
            None => {
                // If no account exists, create a new account and transfer the amount to its
                // purse.
                self.transfer_to_new_account(source, target, amount)
            }
            Some(Value::Account(account)) => {
                let target = account.purse_id_add_only();
                if source == target {
                    return Ok(Ok(TransferredTo::ExistingAccount));
                }
                // If an account exists, transfer the amount to its purse
                self.transfer_to_existing_account(source, target, amount)
            }
            Some(_) => {
                // If some other value exists, return an error
                Err(Error::AccountNotFound(target_key))
            }
        }
    }

    /// Transfers `amount` of motes from `source` purse to `target` purse.
    fn transfer_from_purse_to_purse(
        &mut self,
        source_ptr: u32,
        source_size: u32,
        target_ptr: u32,
        target_size: u32,
        amount_ptr: u32,
        amount_size: u32,
    ) -> Result<Result<(), ApiError>, Error> {
        let source: PurseId = {
            let bytes = self.bytes_from_mem(source_ptr, source_size as usize)?;
            deserialize(&bytes).map_err(Error::BytesRepr)?
        };

        let target: PurseId = {
            let bytes = self.bytes_from_mem(target_ptr, target_size as usize)?;
            deserialize(&bytes).map_err(Error::BytesRepr)?
        };

        let amount: U512 = {
            let bytes = self.bytes_from_mem(amount_ptr, amount_size as usize)?;
            deserialize(&bytes).map_err(Error::BytesRepr)?
        };

        let mint_contract_key = self.get_mint_contract_uref().into();

        if self
            .mint_transfer(mint_contract_key, source, target, amount)
            .is_ok()
        {
            Ok(Ok(()))
        } else {
            Ok(Err(ApiError::Transfer))
        }
    }

    fn get_balance(&mut self, purse_id: PurseId) -> Result<Option<U512>, Error> {
        let seed = self.get_mint_contract_uref().addr();

        let key = purse_id.value().addr().to_bytes()?;

        let uref_key = match self.context.read_ls_with_seed(seed, &key)? {
            Some(Value::Key(uref_key @ Key::URef(_))) => uref_key,
            Some(_) => panic!("expected Value::Key(Key::Uref(_))"),
            None => return Ok(None),
        };

        let ret = match self.context.read_gs_direct(&uref_key)? {
            Some(Value::UInt512(balance)) => Some(balance),
            Some(_) => panic!("expected Value::UInt512(_)"),
            None => None,
        };

        Ok(ret)
    }

    fn get_balance_host_buf(
        &mut self,
        purse_id_ptr: u32,
        purse_id_size: usize,
        output_size_ptr: u32,
    ) -> Result<Result<(), ApiError>, Error> {
        if !self.can_write_to_host_buf() {
            // Exit early if the host buffer is already occupied
            return Ok(Err(ApiError::HostBufferFull));
        }

        let purse_id: PurseId = {
            let bytes = self.bytes_from_mem(purse_id_ptr, purse_id_size)?;
            match deserialize(&bytes) {
                Ok(purse_id) => purse_id,
                Err(error) => return Ok(Err(error.into())),
            }
        };

        let balance = match self.get_balance(purse_id)? {
            Some(balance) => balance,
            None => return Ok(Err(ApiError::InvalidPurse)),
        };

        let balance_bytes = match balance.to_bytes() {
            Ok(bytes) => bytes,
            Err(error) => return Ok(Err(error.into())),
        };

        let balance_size = balance_bytes.len() as i32;
        if let Err(error) = self.write_host_buf(balance_bytes) {
            return Ok(Err(error));
        }

        let balance_size_bytes = balance_size.to_le_bytes(); // wasm is LE
        if let Err(error) = self.memory.set(output_size_ptr, &balance_size_bytes) {
            return Err(Error::Interpreter(error));
        }

        Ok(Ok(()))
    }

    /// If key is in named_keys with AccessRights::Write, processes bytes from calling contract
    /// and writes them at the provided uref, overwriting existing value if any
    pub fn upgrade_contract_at_uref(
        &mut self,
        name_ptr: u32,
        name_size: u32,
        key_ptr: u32,
        key_size: u32,
    ) -> Result<Result<(), ApiError>, Trap> {
        let key = self.key_from_mem(key_ptr, key_size)?;
        let named_keys = match self.context.read_gs(&key)? {
            None => Err(Error::KeyNotFound(key)),
            Some(Value::Contract(contract)) => Ok(contract.named_keys().clone()),
            Some(_) => Err(Error::FunctionNotFound(format!(
                "Value at {:?} is not a contract",
                key
            ))),
        }?;
        let bytes = self.get_function_by_name(name_ptr, name_size)?;
        match self
            .context
            .upgrade_contract_at_uref(key, bytes, named_keys)
        {
            Ok(_) => Ok(Ok(())),
            Err(_) => Ok(Err(ApiError::UpgradeContractAtURef)),
        }
    }

    fn get_system_contract(
        &mut self,
        system_contract_index: u32,
        dest_ptr: u32,
        _dest_size: u32,
    ) -> Result<Result<(), ApiError>, Trap> {
        let attenuated_uref = match SystemContract::try_from(system_contract_index) {
            Ok(SystemContract::Mint) => self.get_mint_contract_uref(),
            Ok(SystemContract::ProofOfStake) => self.get_pos_contract_uref(),
            Err(error) => return Ok(Err(error)),
        };

        // Serialize data that will be written the memory under `dest_ptr`
        let attenuated_uref_bytes = attenuated_uref.to_bytes().map_err(Error::BytesRepr)?;
        match self.memory.set(dest_ptr, &attenuated_uref_bytes) {
            Ok(_) => Ok(Ok(())),
            Err(error) => Err(Error::Interpreter(error).into()),
        }
    }

    /// Takes host buffer data from and returns it and then clears the buffer.
    pub fn take_host_buf(&mut self) -> Option<Vec<u8>> {
        self.host_buf.take()
    }

    /// Checks if a write to host buffer can happen.
    ///
    /// This will check if the host buffer is empty.
    fn can_write_to_host_buf(&self) -> bool {
        self.host_buf.is_none()
    }

    /// Overwrites data in host buffer only if its in empty state
    fn write_host_buf(&mut self, data: Vec<u8>) -> Result<(), ApiError> {
        match self.host_buf {
            Some(_) => return Err(ApiError::HostBufferFull),
            None => self.host_buf = Some(data),
        }
        Ok(())
    }

    fn read_host_buffer(
        &mut self,
        dest_ptr: u32,
        dest_size: usize,
        bytes_written_ptr: u32,
    ) -> Result<Result<(), ApiError>, Error> {
        let host_buf = match self.take_host_buf() {
            None => return Ok(Err(ApiError::HostBufferEmpty)),
            Some(host_buf) => host_buf,
        };
        if host_buf.len() > u32::max_value() as usize {
            return Ok(Err(ApiError::OutOfMemoryError));
        }
        if host_buf.len() > dest_size {
            return Ok(Err(ApiError::BufferTooSmall));
        }

        // Slice data, so if `dest_size` is larger than hostbuf size, it will take host_buf as
        // whole.
        let sliced_buf = &host_buf[..cmp::min(dest_size, host_buf.len())];
        if let Err(error) = self.memory.set(dest_ptr, sliced_buf) {
            return Err(Error::Interpreter(error));
        }

        let bytes_written = sliced_buf.len() as u32;
        let bytes_written_data = bytes_written.to_le_bytes();

        if let Err(error) = self.memory.set(bytes_written_ptr, &bytes_written_data) {
            return Err(Error::Interpreter(error));
        }

        Ok(Ok(()))
    }
}
