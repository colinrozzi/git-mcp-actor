#[allow(warnings)]
mod bindings;

use bindings::exports::theater::simple::actor::Guest;
use bindings::exports::theater::simple::message_server_client::ChannelAccept;
use bindings::exports::theater::simple::message_server_client::Guest as MessageServerClient;
use bindings::exports::theater::simple::supervisor_handlers::Guest as SupervisorHandlers;
use bindings::exports::theater::simple::supervisor_handlers::WitActorError;
use bindings::theater::simple::message_server_host::respond_to_request;
use bindings::theater::simple::runtime::log;
use bindings::theater::simple::supervisor::spawn;
use mcp_protocol::tool::Tool;
use mcp_protocol::tool::ToolCallResult;
use mcp_protocol::tool::ToolContent;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;

struct Component;

const GIT_COMMAND_MANIFEST: &str =
    "https://github.com/colinrozzi/git-command-actor/releases/latest/download/manifest.toml";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct InitState {
    repository_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct State {
    repository_path: Option<String>,
    outstanding_requests: HashMap<String, String>,
}

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

/// Result structure returned on shutdown
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitCommandResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub command: Vec<String>,
    pub execution_time_ms: Option<u64>,
    pub error: Option<String>,
    pub repository_path: String,
}

impl Guest for Component {
    fn init(state: Option<Vec<u8>>, params: (String,)) -> Result<(Option<Vec<u8>>,), String> {
        // Initialize the component with the provided state and parameters
        log(&format!(
            "Initializing with state: {:?} and params: {:?}",
            state, params
        ));

        let init_state = match state {
            Some(state_bytes) if !state_bytes.is_empty() => {
                serde_json::from_slice::<InitState>(&state_bytes)
                    .map_err(|e| format!("Failed to deserialize state: {}", e))?
            }
            _ => InitState {
                repository_path: None,
            },
        };

        let app_state = State {
            outstanding_requests: HashMap::new(),
            repository_path: init_state.repository_path,
        };

        Ok((Some(
            serde_json::to_vec(&app_state).map_err(|e| e.to_string())?,
        ),))
    }
}

impl MessageServerClient for Component {
    fn handle_send(
        state: Option<Vec<u8>>,
        _params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>,), String> {
        log("Handling send message");

        let app_state: State = match state {
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
        let response = match request {
            McpActorRequest::ToolsList {} => {
                log("Received tools_list request");

                let description = match &app_state.repository_path {
                    Some(repo_path) => format!(
                        "Execute a git command in the configured repository: '{}'. Provide 'args' as an array of strings. You can optionally override the repository by providing 'repository_path'. Example: args: ['status', '--porcelain']",
                        repo_path
                    ),
                    None => "Execute a git command. You must provide both 'repository_path' and 'args' as an array of strings. Example: repository_path: '/path/to/repo', args: ['status', '--porcelain']".to_string(),
                };

                let tools = vec![Tool {
                    name: "git-command".to_string(),
                    description: Some(description),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "repository_path": {
                                "type": "string",
                                "description": match &app_state.repository_path {
                                    Some(_) => "Override the configured repository path (optional)",
                                    None => "The path of the git repository (required)",
                                }
                            },
                            "args" : {
                                "type": "array",
                                "items": {
                                    "type": "string"
                                },
                                "description": "Array of command-line arguments to pass to git (e.g., ['status', '--porcelain'] or ['--version'])"
                            }
                        },
                        "required": match &app_state.repository_path {
                            Some(_) => vec![], // No required fields if repo is configured
                            None => vec!["repository_path"], // Require repo path if not configured
                        }
                    }),
                    annotations: None,
                }];

                log(&format!("Available tools: {:?}", tools));
                let res = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request_id,
                    result: Some(json!({
                        "tools": tools
                    })),
                    error: None,
                };
                Some(
                    serde_json::to_vec(&res)
                        .map_err(|e| format!("Failed to serialize response: {}", e))?,
                )
            }

            McpActorRequest::ToolsCall { name, args } => {
                log("Received tools_call request");
                log(&format!("Tool name: {}", name));

                match name.as_str() {
                    "git-command" => {
                        log("Processing git-command call");

                        // Get repository path from call args or fall back to init state
                        let repository_path = match args.get("repository_path").and_then(Value::as_str) {
                            Some(path) => path.to_string(),
                            None => app_state.repository_path.as_ref()
                                .ok_or("No repository path provided in call arguments or initialization state")?
                                .clone(),
                        };

                        let args_array = args.get("args").and_then(Value::as_array).ok_or(
                            "Missing or invalid 'args' argument - expected array of strings",
                        )?;

                        let child_init_state = json!({
                            "repository_path": repository_path,
                            "git_args": args_array
                                .iter()
                                .filter_map(Value::as_str)
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>(),
                        });

                        log(&format!(
                            "Child init state: {}",
                            child_init_state.to_string()
                        ));

                        let child_init_state_bytes = serde_json::to_vec(&child_init_state)
                            .map_err(|e| format!("Failed to serialize child init state: {}", e))?;

                        let actor_id = spawn(GIT_COMMAND_MANIFEST, Some(&child_init_state_bytes))
                            .expect("Failed to spawn git-command actor");

                        app_state
                            .outstanding_requests
                            .insert(actor_id.clone(), request_id.clone());

                        None
                    }
                    _ => {
                        log(&format!("Unknown tool name: {}", name));
                        let err_response = McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request_id,
                            result: None,
                            error: Some(McpError {
                                code: -32601,
                                message: format!("Method '{}' not implemented", name),
                                data: None,
                            }),
                        };

                        log(&format!("Error response: {:?}", err_response));
                        Some(
                            serde_json::to_vec(&err_response).map_err(|e| {
                                format!("Failed to serialize error response: {}", e)
                            })?,
                        )
                    }
                }
            }
        };

        // Serialize the app state
        let updated_state = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;

        // Return updated state
        Ok((Some(updated_state), (response,)))
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

impl SupervisorHandlers for Component {
    fn handle_child_error(
        state: Option<Vec<u8>>,
        params: (String, WitActorError),
    ) -> Result<(Option<Vec<u8>>,), String> {
        log("Handling child error in chat-state");

        let (actor_id, result) = params;

        log(&format!(
            "Child actor {} encountered an error: {:?}",
            actor_id, result
        ));
        let mut app_state: State = match state {
            Some(state_bytes) if !state_bytes.is_empty() => serde_json::from_slice(&state_bytes)
                .map_err(|e| format!("Failed to deserialize state: {}", e))?,
            _ => return Err("Invalid state".to_string()),
        };

        // Check if the actor ID exists in outstanding requests
        let request_id = app_state
            .outstanding_requests
            .get(&actor_id)
            .cloned()
            .ok_or_else(|| format!("No outstanding request found for actor ID {}", actor_id))?;

        // Resolve the outstanding request, passing the error along
        let response = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request_id.clone(),
            result: None,
            error: Some(McpError {
                code: -32000, // Generic error code
                message: format!("Child actor error: {:?}", result.data),
                data: None,
            }),
        };

        log(&format!("Response to outstanding request: {:?}", response));
        let response_bytes = serde_json::to_vec(&response)
            .map_err(|e| format!("Failed to serialize response: {}", e))?;
        respond_to_request(&request_id, &response_bytes).expect("Failed to respond to request");

        // Remove the actor from outstanding requests
        app_state
            .outstanding_requests
            .remove(&actor_id)
            .ok_or_else(|| format!("Actor ID {} not found in outstanding requests", actor_id))?;

        // Serialize the updated state
        let updated_state = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
        Ok((Some(updated_state),))
    }

    fn handle_child_exit(
        state: Option<Vec<u8>>,
        params: (String, Option<Vec<u8>>),
    ) -> Result<(Option<Vec<u8>>,), String> {
        log("Handling child exit in chat-state");

        let (actor_id, result_bytes) = params;

        let result = match result_bytes {
            Some(bytes) => serde_json::from_slice::<GitCommandResult>(&bytes)
                .map_err(|e| format!("Failed to deserialize result: {}", e))?,
            None => GitCommandResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                command: Vec::new(),
                execution_time_ms: None,
                error: Some("Child exited without result".to_string()),
                repository_path: String::new(),
            },
        };

        let mut app_state: State = match state {
            Some(state_bytes) if !state_bytes.is_empty() => serde_json::from_slice(&state_bytes)
                .map_err(|e| format!("Failed to deserialize state: {}", e))?,
            _ => return Err("Invalid state".to_string()),
        };

        log(&format!(
            "Child actor {} exited with result: {:?}",
            actor_id, result
        ));

        let request_id = app_state
            .outstanding_requests
            .remove(&actor_id)
            .ok_or_else(|| format!("No outstanding request found for actor ID {}", actor_id))?;

        // Prepare the response
        let response = match result.success {
            true => {
                let tool_call_result = ToolCallResult {
                    content: vec![ToolContent::Text {
                        text: serde_json::to_string(&result.stdout)
                            .unwrap_or_else(|_| "No stdout".to_string()),
                    }],
                    is_error: None,
                };
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request_id.clone(),
                    result: Some(
                        serde_json::to_value(tool_call_result)
                            .map_err(|e| format!("Failed to serialize result: {}", e))?,
                    ),
                    error: None,
                }
            }
            false => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request_id.clone(),
                result: None,
                error: Some(McpError {
                    code: -32000, // Generic error code
                    message: serde_json::to_string(&result)
                        .unwrap_or_else(|_| "Unknown error".to_string()),
                    data: None,
                }),
            },
        };

        log(&format!("Response to outstanding request: {:?}", response));

        let response_bytes = serde_json::to_vec(&response)
            .map_err(|e| format!("Failed to serialize response: {}", e))?;

        respond_to_request(&request_id, &response_bytes).expect("Failed to respond to request");

        let updated_state = serde_json::to_vec(&app_state)
            .map_err(|e| format!("Failed to serialize updated state: {}", e))?;
        Ok((Some(updated_state),))
    }

    fn handle_child_external_stop(
        state: Option<Vec<u8>>,
        params: (String,),
    ) -> Result<(Option<Vec<u8>>,), String> {
        log("Handling child external stop in chat-state");
        let actor_id = params.0;
        log(&format!("Child actor {} requested external stop", actor_id));
        Ok((state,))
    }
}

bindings::export!(Component with_types_in bindings);
