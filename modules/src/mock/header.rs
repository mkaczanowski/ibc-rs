use std::convert::{TryFrom, TryInto};

use serde_derive::{Deserialize, Serialize};
use tendermint_proto::Protobuf;

use ibc_proto::ibc::mock::Header as RawMockHeader;

use crate::ics02_client::client_consensus::AnyConsensusState;
use crate::ics02_client::client_type::ClientType;
use crate::ics02_client::error::{self, Error};
use crate::ics02_client::header::AnyHeader;
use crate::ics02_client::header::Header;
use crate::mock::client_state::MockConsensusState;
use crate::Height;

#[derive(Copy, Clone, Default, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct MockHeader {
    pub height: Height,
    pub timestamp: u64,
}

impl Protobuf<RawMockHeader> for MockHeader {}

impl TryFrom<RawMockHeader> for MockHeader {
    type Error = Error;

    fn try_from(raw: RawMockHeader) -> Result<Self, Self::Error> {
        Ok(MockHeader {
            height: raw
                .height
                .ok_or_else(|| error::Kind::InvalidRawHeader.context("missing height in header"))?
                .try_into()
                .map_err(|e| error::Kind::InvalidRawHeader.context(e))?,
            timestamp: raw.timestamp,
        })
    }
}

impl From<MockHeader> for RawMockHeader {
    fn from(value: MockHeader) -> Self {
        value.into()
    }
}

impl MockHeader {
    pub fn height(&self) -> Height {
        self.height
    }
    pub fn new(height: Height) -> Self {
        Self {
            height,
            timestamp: Default::default(),
        }
    }
}

impl From<MockHeader> for AnyHeader {
    fn from(mh: MockHeader) -> Self {
        Self::Mock(mh)
    }
}

impl Header for MockHeader {
    fn client_type(&self) -> ClientType {
        ClientType::Mock
    }

    fn height(&self) -> Height {
        todo!()
    }

    fn wrap_any(self) -> AnyHeader {
        todo!()
    }
}

impl From<MockHeader> for AnyConsensusState {
    fn from(h: MockHeader) -> Self {
        AnyConsensusState::Mock(MockConsensusState(h))
    }
}
