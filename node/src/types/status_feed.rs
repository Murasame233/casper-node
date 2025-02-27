use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use casper_binary_port::ConsensusStatus;
use casper_types::{
    ActivationPoint, AvailableBlockRange, Block, BlockHash, BlockSynchronizerStatus, Digest, EraId,
    NextUpgrade, Peers, ProtocolVersion, PublicKey, TimeDiff, Timestamp,
};

use crate::{
    components::rest_server::{DocExample, DOCS_EXAMPLE_PROTOCOL_VERSION},
    reactor::main_reactor::ReactorState,
    types::NodeId,
};

static CHAINSPEC_INFO: Lazy<ChainspecInfo> = Lazy::new(|| {
    let next_upgrade = NextUpgrade::new(
        ActivationPoint::EraId(EraId::from(42)),
        ProtocolVersion::from_parts(2, 0, 1),
    );
    ChainspecInfo {
        name: String::from("casper-example"),
        next_upgrade: Some(next_upgrade),
    }
});

static GET_STATUS_RESULT: Lazy<GetStatusResult> = Lazy::new(|| {
    let node_id = NodeId::doc_example();
    let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 54321);
    let mut peers = BTreeMap::new();
    peers.insert(*node_id, socket_addr.to_string());
    let status_feed = StatusFeed {
        last_added_block: Some(Block::example().clone()),
        peers,
        chainspec_info: ChainspecInfo::doc_example().clone(),
        our_public_signing_key: Some(PublicKey::example().clone()),
        round_length: Some(TimeDiff::from_millis(1 << 16)),
        version: crate::VERSION_STRING.as_str(),
        node_uptime: Duration::from_secs(13),
        reactor_state: ReactorState::Initialize,
        last_progress: Timestamp::from(0),
        available_block_range: AvailableBlockRange::RANGE_0_0,
        block_sync: BlockSynchronizerStatus::example().clone(),
        starting_state_root_hash: Digest::default(),
        latest_switch_block_hash: Some(BlockHash::default()),
    };
    GetStatusResult::new(status_feed, DOCS_EXAMPLE_PROTOCOL_VERSION)
});

/// Summary information from the chainspec.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainspecInfo {
    /// Name of the network.
    name: String,
    next_upgrade: Option<NextUpgrade>,
}

impl DocExample for ChainspecInfo {
    fn doc_example() -> &'static Self {
        &CHAINSPEC_INFO
    }
}

impl ChainspecInfo {
    pub(crate) fn new(chainspec_network_name: String, next_upgrade: Option<NextUpgrade>) -> Self {
        ChainspecInfo {
            name: chainspec_network_name,
            next_upgrade,
        }
    }
}

/// Data feed for client "info_get_status" endpoint.
#[derive(Debug, Serialize)]
pub struct StatusFeed {
    /// The last block added to the chain.
    pub last_added_block: Option<Block>,
    /// The peer nodes which are connected to this node.
    pub peers: BTreeMap<NodeId, String>,
    /// The chainspec info for this node.
    pub chainspec_info: ChainspecInfo,
    /// Our public signing key.
    pub our_public_signing_key: Option<PublicKey>,
    /// The next round length if this node is a validator.
    pub round_length: Option<TimeDiff>,
    /// The compiled node version.
    pub version: &'static str,
    /// Time that passed since the node has started.
    pub node_uptime: Duration,
    /// The current state of node reactor.
    pub reactor_state: ReactorState,
    /// Timestamp of the last recorded progress in the reactor.
    pub last_progress: Timestamp,
    /// The available block range in storage.
    pub available_block_range: AvailableBlockRange,
    /// The status of the block synchronizer builders.
    pub block_sync: BlockSynchronizerStatus,
    /// The state root hash of the lowest block in the available block range.
    pub starting_state_root_hash: Digest,
    /// The hash of the latest switch block.
    pub latest_switch_block_hash: Option<BlockHash>,
}

impl StatusFeed {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        last_added_block: Option<Block>,
        peers: BTreeMap<NodeId, String>,
        chainspec_info: ChainspecInfo,
        consensus_status: Option<ConsensusStatus>,
        node_uptime: Duration,
        reactor_state: ReactorState,
        last_progress: Timestamp,
        available_block_range: AvailableBlockRange,
        block_sync: BlockSynchronizerStatus,
        starting_state_root_hash: Digest,
        latest_switch_block_hash: Option<BlockHash>,
    ) -> Self {
        let (our_public_signing_key, round_length) =
            consensus_status.map_or((None, None), |consensus_status| {
                (
                    Some(consensus_status.validator_public_key().clone()),
                    consensus_status.round_length(),
                )
            });
        StatusFeed {
            last_added_block,
            peers,
            chainspec_info,
            our_public_signing_key,
            round_length,
            version: crate::VERSION_STRING.as_str(),
            node_uptime,
            reactor_state,
            last_progress,
            available_block_range,
            block_sync,
            starting_state_root_hash,
            latest_switch_block_hash,
        }
    }
}

/// Minimal info of a `Block`.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MinimalBlockInfo {
    hash: BlockHash,
    timestamp: Timestamp,
    era_id: EraId,
    height: u64,
    state_root_hash: Digest,
    creator: PublicKey,
}

impl From<Block> for MinimalBlockInfo {
    fn from(block: Block) -> Self {
        let proposer = match &block {
            Block::V1(v1) => v1.proposer().clone(),
            Block::V2(v2) => v2.proposer().clone(),
        };

        MinimalBlockInfo {
            hash: *block.hash(),
            timestamp: block.timestamp(),
            era_id: block.era_id(),
            height: block.height(),
            state_root_hash: *block.state_root_hash(),
            creator: proposer,
        }
    }
}

/// Result for "info_get_status" RPC response.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetStatusResult {
    /// The node ID and network address of each connected peer.
    pub peers: Peers,
    /// The RPC API version.
    #[schemars(with = "String")]
    pub api_version: ProtocolVersion,
    /// The compiled node version.
    pub build_version: String,
    /// The chainspec name.
    pub chainspec_name: String,
    /// The state root hash of the lowest block in the available block range.
    pub starting_state_root_hash: Digest,
    /// The minimal info of the last block from the linear chain.
    pub last_added_block_info: Option<MinimalBlockInfo>,
    /// Our public signing key.
    pub our_public_signing_key: Option<PublicKey>,
    /// The next round length if this node is a validator.
    pub round_length: Option<TimeDiff>,
    /// Information about the next scheduled upgrade.
    pub next_upgrade: Option<NextUpgrade>,
    /// Time that passed since the node has started.
    pub uptime: TimeDiff,
    /// The current state of node reactor.
    pub reactor_state: ReactorState,
    /// Timestamp of the last recorded progress in the reactor.
    pub last_progress: Timestamp,
    /// The available block range in storage.
    pub available_block_range: AvailableBlockRange,
    /// The status of the block synchronizer builders.
    pub block_sync: BlockSynchronizerStatus,
    /// The hash of the latest switch block.
    pub latest_switch_block_hash: Option<BlockHash>,
}

impl GetStatusResult {
    #[allow(deprecated)]
    pub(crate) fn new(status_feed: StatusFeed, api_version: ProtocolVersion) -> Self {
        GetStatusResult {
            peers: Peers::from(status_feed.peers),
            api_version,
            chainspec_name: status_feed.chainspec_info.name,
            starting_state_root_hash: status_feed.starting_state_root_hash,
            last_added_block_info: status_feed.last_added_block.map(Into::into),
            our_public_signing_key: status_feed.our_public_signing_key,
            round_length: status_feed.round_length,
            next_upgrade: status_feed.chainspec_info.next_upgrade,
            uptime: status_feed.node_uptime.into(),
            reactor_state: status_feed.reactor_state,
            last_progress: status_feed.last_progress,
            available_block_range: status_feed.available_block_range,
            block_sync: status_feed.block_sync,
            latest_switch_block_hash: status_feed.latest_switch_block_hash,
            #[cfg(not(test))]
            build_version: crate::VERSION_STRING.clone(),

            //  Prevent these values from changing between test sessions
            #[cfg(test)]
            build_version: String::from("1.0.0-xxxxxxxxx@DEBUG"),
        }
    }
}

impl DocExample for GetStatusResult {
    fn doc_example() -> &'static Self {
        &GET_STATUS_RESULT
    }
}
