//! Adapter module for connecting icn_core and icn_network types
//!
//! This module provides conversion functions between the core and network types,
//! particularly for network messages.

use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use serde_json::{json, Value};

use icn_core::networking::NetworkMessage as CoreNetworkMessage;
use crate::NetworkMessage;
use crate::{
    LedgerStateUpdate, 
    TransactionAnnouncement, 
    IdentityAnnouncement, 
    ProposalAnnouncement, 
    VoteAnnouncement, 
    CustomMessage
};

/// Convert from a Core NetworkMessage to a Network crate NetworkMessage
pub fn core_to_network_message(core_msg: CoreNetworkMessage) -> NetworkMessage {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let payload_json: Value = serde_json::from_slice(&core_msg.payload)
        .unwrap_or_else(|_| json!({}));
    
    match core_msg.message_type.as_str() {
        "ledger.state" => {
            let update = LedgerStateUpdate {
                ledger_hash: payload_json["ledger_hash"].as_str().unwrap_or("").to_string(),
                transaction_count: payload_json["transaction_count"].as_u64().unwrap_or(0),
                account_count: payload_json["account_count"].as_u64().unwrap_or(0),
                transaction_ids: payload_json["transaction_ids"].as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_else(Vec::new),
                timestamp,
            };
            NetworkMessage::LedgerStateUpdate(update)
        },
        "ledger.transaction" => {
            let tx = TransactionAnnouncement {
                transaction_id: payload_json["transaction_id"].as_str().unwrap_or("").to_string(),
                transaction_type: payload_json["transaction_type"].as_str().unwrap_or("").to_string(),
                timestamp,
                sender: payload_json["sender"].as_str().unwrap_or("").to_string(),
                data_hash: payload_json["data_hash"].as_str().unwrap_or("").to_string(),
            };
            NetworkMessage::TransactionAnnouncement(tx)
        },
        "identity.announcement" => {
            let id = IdentityAnnouncement {
                identity_id: payload_json["identity_id"].as_str().unwrap_or("").to_string(),
                public_key: payload_json["public_key"].as_str()
                    .map(|s| s.as_bytes().to_vec())
                    .unwrap_or_default(),
                metadata: payload_json["metadata"].as_object()
                    .map(|obj| obj.iter().filter_map(|(k, v)| {
                        v.as_str().map(|s| (k.clone(), s.to_string()))
                    }).collect())
                    .unwrap_or_else(HashMap::new),
                timestamp,
            };
            NetworkMessage::IdentityAnnouncement(id)
        },
        "governance.proposal" => {
            let prop = ProposalAnnouncement {
                proposal_id: payload_json["proposal_id"].as_str().unwrap_or("").to_string(),
                title: payload_json["title"].as_str().unwrap_or("").to_string(),
                author: payload_json["author"].as_str().unwrap_or("").to_string(),
                timestamp,
                voting_ends_at: payload_json["voting_ends_at"].as_u64().unwrap_or(0),
                data_hash: payload_json["data_hash"].as_str().unwrap_or("").to_string(),
            };
            NetworkMessage::ProposalAnnouncement(prop)
        },
        "governance.vote" => {
            let vote = VoteAnnouncement {
                proposal_id: payload_json["proposal_id"].as_str().unwrap_or("").to_string(),
                voter_id: payload_json["voter_id"].as_str().unwrap_or("").to_string(),
                decision: payload_json["decision"].as_str().unwrap_or("").to_string(),
                timestamp,
                data_hash: payload_json["data_hash"].as_str().unwrap_or("").to_string(),
            };
            NetworkMessage::VoteAnnouncement(vote)
        },
        _ => {
            // Treat as custom message
            let mut data = serde_json::Map::new();
            if let Some(obj) = payload_json.as_object() {
                for (key, value) in obj {
                    data.insert(key.clone(), value.clone());
                }
            }
            
            let custom = CustomMessage {
                message_type: core_msg.message_type,
                data,
            };
            NetworkMessage::Custom(custom)
        }
    }
}

/// Convert from a Network crate NetworkMessage to a Core NetworkMessage
pub fn network_to_core_message(network_msg: NetworkMessage, sender: &str) -> CoreNetworkMessage {
    let (message_type, payload) = match network_msg {
        NetworkMessage::LedgerStateUpdate(update) => {
            let payload = json!({
                "ledger_hash": update.ledger_hash,
                "transaction_count": update.transaction_count,
                "account_count": update.account_count,
                "transaction_ids": update.transaction_ids,
                "timestamp": update.timestamp,
            });
            
            ("ledger.state".to_string(), serde_json::to_vec(&payload).unwrap_or_default())
        },
        NetworkMessage::TransactionAnnouncement(tx) => {
            let payload = json!({
                "transaction_id": tx.transaction_id,
                "transaction_type": tx.transaction_type,
                "sender": tx.sender,
                "data_hash": tx.data_hash,
                "timestamp": tx.timestamp,
            });
            
            ("ledger.transaction".to_string(), serde_json::to_vec(&payload).unwrap_or_default())
        },
        NetworkMessage::IdentityAnnouncement(id) => {
            let payload = json!({
                "identity_id": id.identity_id,
                "public_key": String::from_utf8_lossy(&id.public_key),
                "metadata": id.metadata,
                "timestamp": id.timestamp,
            });
            
            ("identity.announcement".to_string(), serde_json::to_vec(&payload).unwrap_or_default())
        },
        NetworkMessage::ProposalAnnouncement(prop) => {
            let payload = json!({
                "proposal_id": prop.proposal_id,
                "title": prop.title,
                "author": prop.author,
                "timestamp": prop.timestamp,
                "voting_ends_at": prop.voting_ends_at,
                "data_hash": prop.data_hash,
            });
            
            ("governance.proposal".to_string(), serde_json::to_vec(&payload).unwrap_or_default())
        },
        NetworkMessage::VoteAnnouncement(vote) => {
            let payload = json!({
                "proposal_id": vote.proposal_id,
                "voter_id": vote.voter_id,
                "decision": vote.decision,
                "timestamp": vote.timestamp,
                "data_hash": vote.data_hash,
            });
            
            ("governance.vote".to_string(), serde_json::to_vec(&payload).unwrap_or_default())
        },
        NetworkMessage::Custom(custom) => {
            (custom.message_type, serde_json::to_vec(&custom.data).unwrap_or_default())
        }
    };
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    CoreNetworkMessage {
        message_type,
        payload,
        sender: sender.to_string(),
        recipient: None,
        timestamp,
    }
} 