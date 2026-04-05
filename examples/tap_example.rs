// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! Tap Example — media tapping and streaming actions.

use signalwire::swaig::FunctionResult;
use serde_json::json;

fn main() {
    // Basic RTP tap
    println!("=== Basic RTP Tap ===");
    let mut result = FunctionResult::with_response("Starting audio tap.");
    result.add_action(json!({
        "tap": {
            "type": "audio",
            "params": {
                "direction": "both",
                "codec": "PCMU",
                "rate": 8000
            },
            "target": {
                "type": "rtp",
                "params": {
                    "addr": "192.168.1.100",
                    "port": 9000
                }
            }
        }
    }));
    println!("{}", result.to_json());

    // WebSocket tap for real-time processing
    println!("\n=== WebSocket Tap ===");
    let mut result = FunctionResult::with_response("Starting WebSocket audio stream.");
    result.add_action(json!({
        "tap": {
            "type": "audio",
            "params": {
                "direction": "both",
                "codec": "PCMU",
                "rate": 16000
            },
            "target": {
                "type": "ws",
                "params": {
                    "uri": "wss://analytics.example.com/stream"
                }
            }
        }
    }));
    println!("{}", result.to_json());

    // Tap with metadata and global data update
    println!("\n=== Tap with Metadata ===");
    let mut result = FunctionResult::with_response("Starting monitored tap.");
    result.add_action(json!({
        "set_metadata": {
            "tap_reason": "quality_monitoring",
            "tap_id": "tap_20240115_001"
        }
    }));
    result.add_action(json!({
        "tap": {
            "type": "audio",
            "params": {"direction": "speak", "codec": "PCMU", "rate": 8000},
            "target": {
                "type": "rtp",
                "params": {"addr": "10.0.0.50", "port": 9001}
            }
        }
    }));
    result.add_action(json!({"update_global_data": {"tap_active": true}}));
    println!("{}", result.to_json());
}
