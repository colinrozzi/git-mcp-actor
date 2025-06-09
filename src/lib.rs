#[allow(warnings)]
mod bindings;

use bindings::exports::theater::simple::actor::Guest;
use bindings::exports::theater::simple::message_server_client::ChannelAccept;
use bindings::exports::theater::simple::message_server_client::Guest as MessageServerClient;
use bindings::theater::simple::runtime::log;
use mcp_protocol::tool::Tool;
use mcp_protocol::tool::ToolCallResult;
use mcp_protocol::tool::ToolContent;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::json;

struct Component;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct State;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct McpRequest {
    jsonrpc: String,
    id: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

// Actor API request structures
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum McpActorRequest {
    ToolsList {},
    ToolsCall { name: String, args: Value },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct McpResponse {
    jsonrpc: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct McpError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl Guest for Component {
    fn init(state: Option<Vec<u8>>, params: (String,)) -> Result<(Option<Vec<u8>>,), String> {
        // Initialize the component with the provided state and parameters
        println!(
            "Initializing with state: {:?} and params: {:?}",
            state, params
        );

        // Return the updated state
        Ok((state,)) // Returning the same state for simplicity
    }
}

impl MessageServerClient for Component {
    fn handle_send(
        state: Option<Vec<u8>>,
        params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>,), String> {
        log("Handling send message");

        let mut app_state: State = match state {
            Some(state_bytes) if !state_bytes.is_empty() => serde_json::from_slice(&state_bytes)
                .map_err(|e| format!("Failed to deserialize state: {}", e))?,
            _ => return Err("Invalid state".to_string()),
        };

        let state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
        Ok((Some(state_bytes),))
    }

    fn handle_request(
        state: Option<Vec<u8>>,
        params: (String, Vec<u8>),
    ) -> Result<(Option<Vec<u8>>, (Option<Vec<u8>>,)), String> {
        log("Handling request message");
        log("new version");
        let (request_id, request) = params;
        log(&format!("Request ID: {}", request_id));
        log(&format!("Request data: {:?}", request));

        // Parse the current state
        let mut app_state: State = match state {
            Some(state_bytes) if !state_bytes.is_empty() => serde_json::from_slice(&state_bytes)
                .map_err(|e| format!("Failed to deserialize state: {}", e))?,
            _ => return Err("Invalid state".to_string()),
        };

        // Parse the request
        let request = match serde_json::from_slice::<McpActorRequest>(&request) {
            Ok(req) => req,
            Err(e) => {
                log(&format!("Failed to parse request: {}", e));
                return Err("Unknown request format".to_string());
            }
        };

        // Process the request based on its type
        let mcp_response = match request {
            McpActorRequest::ToolsList {} => {
                log("Received tools_list request");

                let tools = vec![Tool {
                    name: "example_tool".to_string(),
                    description: Some("An example tool".to_string()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "example_param": {
                                "type": "string",
                                "description": "An example parameter"
                            }
                        },
                        "required": ["example_param"]
                    }),
                    annotations: None,
                }];

                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request_id,
                    result: Some(json!({
                        "tools": tools
                    })),
                    error: None,
                }
            }

            McpActorRequest::ToolsCall { name, args } => {
                log("Received tools_call request");
                log(&format!("Tool name: {}", name));

                match name.as_str() {
                    "example_tool" => {
                        log("Processing example_tool call");

                        let result = ToolCallResult {
                            content: vec![ToolContent::Text {
                                text: "This is an example response from the tool.".to_string(),
                            }],
                            is_error: None,
                        };

                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request_id,
                            result: Some(
                                serde_json::to_value(&result)
                                    .map_err(|e| format!("Failed to serialize result: {}", e))
                                    .expect("Serialization of tools call result should not fail"),
                            ),
                            error: None,
                        }
                    }
                    _ => {
                        log(&format!("Unknown tool name: {}", name));
                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request_id,
                            result: None,
                            error: Some(McpError {
                                code: -32601,
                                message: format!("Method '{}' not implemented", name),
                                data: None,
                            }),
                        }
                    }
                }
            }
        };

        log(&format!("Response: {:?}", mcp_response));

        // Serialize the app state
        let updated_state = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
        let response_bytes = serde_json::to_vec(&mcp_response).map_err(|e| e.to_string())?;

        // Return updated state
        Ok((Some(updated_state), (Some(response_bytes),)))
    }

    fn handle_channel_open(
        state: Option<Vec<u8>>,
        _params: (String, Vec<u8>),
    ) -> Result<(Option<Vec<u8>>, (ChannelAccept,)), String> {
        Ok((
            state,
            (ChannelAccept {
                accepted: true,
                message: None,
            },),
        ))
    }

    fn handle_channel_close(
        state: Option<Vec<u8>>,
        _params: (String,),
    ) -> Result<(Option<Vec<u8>>,), String> {
        Ok((state,))
    }

    fn handle_channel_message(
        state: Option<Vec<u8>>,
        _params: (String, Vec<u8>),
    ) -> Result<(Option<Vec<u8>>,), String> {
        log("mcp-actor: Received channel message");
        Ok((state,))
    }
}

bindings::export!(Component with_types_in bindings);
