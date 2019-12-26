use hex_fmt::HexFmt;

use core::{any::type_name, fmt, marker::PhantomData};

use crate::{
    uref::{AccessRights, URef},
    value::CLTyped,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AccessRightsError {
    NoAccessRights,
}

impl fmt::Display for AccessRightsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            AccessRightsError::NoAccessRights => write!(f, "URef has no access rights"),
        }
    }
}

// TODO: TURef might needs to be encoded into more fine grained types
// rather than hold AccessRights as one of the fields in order to be able
// to statically provide guarantees about how it can operate on the keys.

// URef with type information about what value is in the global state
#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub struct TURef<T> {
    addr: [u8; 32],
    access_rights: AccessRights,
    _marker: PhantomData<T>,
}

impl<T: CLTyped> TURef<T> {
    pub fn new(addr: [u8; 32], access_rights: AccessRights) -> Self {
        TURef {
            addr,
            access_rights,
            _marker: PhantomData,
        }
    }

    pub fn from_uref(uref: URef) -> Result<Self, AccessRightsError> {
        if let Some(access_rights) = uref.access_rights() {
            let addr = uref.addr();
            Ok(TURef {
                addr,
                access_rights,
                _marker: PhantomData,
            })
        } else {
            Err(AccessRightsError::NoAccessRights)
        }
    }

    pub fn addr(&self) -> [u8; 32] {
        self.addr
    }

    pub fn access_rights(&self) -> AccessRights {
        self.access_rights
    }

    pub fn set_access_rights(&mut self, access_rights: AccessRights) {
        self.access_rights = access_rights;
    }
}

impl<T> core::fmt::Display for TURef<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "TURef({}, {}; {})",
            HexFmt(&self.addr),
            self.access_rights,
            type_name::<T>()
        )
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::{String, ToString};

    use crate::{contract_api::TURef, key::Key, uref::AccessRights};

    #[test]
    fn turef_as_string() {
        let addr_array = [48u8; 32];
        {
            let turef: TURef<String> = TURef::new(addr_array, AccessRights::ADD);
            assert_eq!(
                turef.to_string(),
                "TURef(3030303030303030303030303030303030303030303030303030303030303030, ADD; alloc::string::String)"
            );
        }

        {
            let turef: TURef<Key> = TURef::new(addr_array, AccessRights::READ_ADD_WRITE);
            assert_eq!(
                turef.to_string(),
                "TURef(3030303030303030303030303030303030303030303030303030303030303030, READ_ADD_WRITE; casperlabs_contract_ffi::key::Key)"
            );
        }
    }
}
