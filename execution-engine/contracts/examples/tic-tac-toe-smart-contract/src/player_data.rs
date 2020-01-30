use alloc::{string::String, vec::Vec};
use core::convert::TryInto;

use contract::{
    contract_api::{storage, TURef},
    unwrap_or_revert::UnwrapOrRevert,
};
use num_traits::{FromPrimitive, ToPrimitive};

use tic_tac_toe_logic::player::Player;
use types::{
    account::PublicKey,
    bytesrepr::{self, FromBytes, ToBytes},
    AccessRights, CLType, CLTyped,
};

use crate::error::Error;

const PLAYER_DATA_BYTES_SIZE: usize = 1 + 32 + 32;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct PlayerData {
    piece: Player,
    opponent: PublicKey,
    status_key: TURef<String>,
}

impl PlayerData {
    pub fn read_local(key: PublicKey) -> Option<PlayerData> {
        storage::read_local(&key).unwrap_or_revert_with(Error::PlayerDataDeserialization)
    }

    pub fn write_local(
        key: PublicKey,
        piece: Player,
        opponent: PublicKey,
        status_key: TURef<String>,
    ) {
        let data = PlayerData {
            piece,
            opponent,
            status_key,
        };

        storage::write_local(key, data);
    }

    pub fn piece(&self) -> Player {
        self.piece
    }

    pub fn opponent(&self) -> PublicKey {
        self.opponent
    }

    pub fn status_key(&self) -> TURef<String> {
        self.status_key.clone()
    }
}

impl CLTyped for PlayerData {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl ToBytes for PlayerData {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut result = Vec::with_capacity(PLAYER_DATA_BYTES_SIZE);
        result.push(self.piece.to_u8().unwrap());
        for byte in self
            .opponent
            .value()
            .iter()
            .chain(self.status_key.addr().iter())
        {
            result.push(*byte);
        }

        Ok(result)
    }
}

impl FromBytes for PlayerData {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let piece = FromPrimitive::from_u8(bytes[0]).ok_or(bytesrepr::Error::EarlyEndOfStream)?;
        let opponent_key: [u8; 32] = bytes[1..33]
            .try_into()
            .map_err(|_| bytesrepr::Error::FormattingError)?;
        let status_key: [u8; 32] = bytes[33..]
            .try_into()
            .map_err(|_| bytesrepr::Error::FormattingError)?;
        let opponent = PublicKey::new(opponent_key);
        let status_key = TURef::new(status_key, AccessRights::READ_ADD_WRITE);
        Ok((
            PlayerData {
                piece,
                opponent,
                status_key,
            },
            &[],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::PlayerData;
    use contract::contract_api::TURef;
    use types::{
        account::PublicKey,
        bytesrepr::{FromBytes, ToBytes},
        AccessRights,
    };

    use tic_tac_toe_logic::player::Player;

    #[test]
    fn player_data_round_trip() {
        let player_data = PlayerData {
            piece: Player::X,
            opponent: PublicKey::new([3u8; 32]),
            status_key: TURef::new([5u8; 32], AccessRights::READ_ADD_WRITE),
        };
        let value = player_data.to_bytes().expect("Should serialize");
        let player_data_2: (PlayerData, &[u8]) =
            PlayerData::from_bytes(&value).expect("Should deserialize");
        assert_eq!(player_data, player_data_2.0);
    }
}
