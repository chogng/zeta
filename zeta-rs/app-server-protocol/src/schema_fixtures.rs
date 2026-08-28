use super::*;
use crate::protocol::config::ExecPolicyRuleUpsertParams;
use crate::protocol::config::{
    ApprovalReviewModelSelectionDto, ConfigUpdateParams, McpServerUpsertParams,
    SkillSourceAddParams,
};
use crate::protocol::fs::FsChanged;
use crate::protocol::registry::{CLIENT_METHODS, HOST_METHODS, SERVER_NOTIFICATIONS};
use crate::protocol::slash_commands::{SlashCommandArgumentModeDto, SlashCommandDefinition};
use crate::protocol::turn::InputItem;
use crate::rpc::{JsonRpcFailure, JsonRpcId, JsonRpcNotification, JsonRpcRequest, JsonRpcSuccess};
use std::collections::BTreeSet;
use zeta_protocol::ContentDigest;
use zeta_protocol::Patch;
use zeta_protocol::SessionEvent;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillRef;
use zeta_protocol::SkillSourceId;
use zeta_protocol::ThreadEvent;

#[test]
fn registry_method_and_notification_names_are_unique() {
    let methods = CLIENT_METHODS
        .iter()
        .map(|definition| definition.method)
        .collect::<BTreeSet<_>>();
    let notifications = SERVER_NOTIFICATIONS
        .iter()
        .map(|definition| definition.method)
        .collect::<BTreeSet<_>>();
    let host_methods = HOST_METHODS
        .iter()
        .map(|definition| definition.method)
        .collect::<BTreeSet<_>>();

    assert_eq!(methods.len(), CLIENT_METHODS.len());
    assert_eq!(notifications.len(), SERVER_NOTIFICATIONS.len());
    assert_eq!(host_methods.len(), HOST_METHODS.len());
    assert!(methods.contains("initialize"));
    assert!(methods.contains("session/request"));
    assert!(methods.contains("session/create"));
    assert!(methods.contains("session/thread/read"));
    assert!(methods.contains("session/thread/subscribe"));
    assert!(methods.contains("session/thread/unsubscribe"));
    for obsolete in [
        "session/thread/create",
        "session/thread/fork",
        "session/thread/rewind",
        "session/thread/archive",
        "session/complete",
        "session/archive",
        "session/stop",
        "session/model/set",
        "thread/read",
        "thread/subscribe",
        "thread/unsubscribe",
        "turn/start",
        "turn/shell/start",
        "turn/interrupt",
        "turn/interaction/resolve",
    ] {
        assert!(
            !methods.contains(obsolete),
            "obsolete method remains registered: {obsolete}"
        );
    }
    assert!(methods.contains("document/typst/compile"));
    assert!(methods.contains("fs/getMetadata"));
    assert!(methods.contains("fs/readDirectory"));
    assert!(methods.contains("fs/readFile"));
    assert!(methods.contains("fs/readBinaryFile"));
    assert!(methods.contains("fs/writeFile"));
    assert!(methods.contains("git/status"));
    assert!(methods.contains("git/repositories"));
    assert!(methods.contains("git/textDiff"));
    assert!(methods.contains("git/branch/list"));
    assert!(methods.contains("git/branch/switch"));
    assert!(methods.contains("git/stage"));
    assert!(methods.contains("git/unstage"));
    assert!(methods.contains("git/discardWorktree"));
    assert!(methods.contains("git/commit"));
    assert!(methods.contains("git/fetch"));
    assert!(methods.contains("git/pull"));
    assert!(methods.contains("git/push"));
    assert!(methods.contains("workspace/search/start"));
    assert!(methods.contains("workspace/search/read"));
    assert!(methods.contains("workspace/search/cancel"));
    assert!(methods.contains("workspace/codeIndex/status"));
    assert!(methods.contains("workspace/codeIndex/search"));
    assert!(methods.contains("workspace/codeIndex/retrieve"));
    assert!(methods.contains("workspace/codeIndex/rebuild"));
    assert!(methods.contains("terminal/profile/list"));
    assert!(methods.contains("terminal/create"));
    assert!(methods.contains("terminal/attach"));
    assert!(methods.contains("terminal/write"));
    assert!(methods.contains("terminal/resize"));
    assert!(methods.contains("terminal/read"));
    assert!(methods.contains("terminal/close"));
    assert!(methods.contains("plugin/request/upsert"));
    assert!(methods.contains("hook/upsert"));
    assert!(notifications.contains("session/update"));
    assert!(notifications.contains("session/thread/update"));
    assert!(notifications.contains("agent/request"));
    assert!(!notifications.contains("thread/update"));
    assert!(notifications.contains("git/statusChanged"));
    assert!(notifications.contains("fs/changed"));
    assert_eq!(
        host_methods,
        BTreeSet::from([
            "browser/close",
            "browser/create",
            "browser/observe",
            "browser/perform",
        ])
    );
}

#[test]
fn turn_input_items_preserve_ordered_text_context_image_and_skill_shapes() {
    let input = vec![
        InputItem::Text {
            text: "describe".into(),
        },
        InputItem::Context {
            name: "Git commit abc1234".into(),
            content: "diff --git a/file b/file".into(),
        },
        InputItem::Image {
            url: "https://example.test/image.png".into(),
        },
        InputItem::Skill {
            skill: SkillRef::pinned(
                SkillId::new(
                    SkillSourceId::new("user:skill-source:personal").unwrap(),
                    SkillName::new("review").unwrap(),
                ),
                ContentDigest::sha256(b"skill"),
            ),
        },
    ];

    assert_eq!(
        serde_json::to_value(input).unwrap(),
        serde_json::json!([
            {"type": "text", "text": "describe"},
            {
                "type": "context",
                "name": "Git commit abc1234",
                "content": "diff --git a/file b/file"
            },
            {"type": "image", "url": "https://example.test/image.png"},
            {
                "type": "skill",
                "skill": {
                    "id": {
                        "source": "user:skill-source:personal",
                        "name": "review"
                    },
                    "version": {
                        "type": "pinnedDigest",
                        "digest": "sha256:9c53c074d7ac6a2728b638ac1f376c5fa9eb8f71603017c3ea638c2fd40548df"
                    }
                }
            }
        ])
    );
}

#[test]
fn filesystem_change_hints_are_relative_or_request_a_rescan() {
    assert_eq!(
        serde_json::to_value(FsChanged::PathsChanged {
            workspace_folder_id: None,
            paths: vec!["src/lib.rs".into()],
        })
        .unwrap(),
        serde_json::json!({"type":"pathsChanged","paths":["src/lib.rs"]}),
    );
    assert_eq!(
        serde_json::to_value(FsChanged::RescanRequired {
            workspace_folder_id: None
        })
        .unwrap(),
        serde_json::json!({"type":"rescanRequired"}),
    );
}

#[test]
fn slash_command_definition_preserves_discovery_and_argument_shape() {
    let definition = SlashCommandDefinition {
        name: "diagnose".into(),
        description: "inspect the current workspace".into(),
        argument_mode: SlashCommandArgumentModeDto::Optional,
    };

    assert_eq!(
        serde_json::to_value(definition).unwrap(),
        serde_json::json!({
            "name": "diagnose",
            "description": "inspect the current workspace",
            "argumentMode": "optional"
        })
    );
}

#[test]
fn rpc_envelopes_preserve_json_rpc_2_shape() {
    let request = JsonRpcRequest::new(
        JsonRpcId::Number(7),
        "session/list".into(),
        serde_json::json!({}),
    );
    let notification = JsonRpcNotification::new(
        "session/thread/update".into(),
        serde_json::json!({
            "sessionId": "session-1",
            "threadId": "thread-1",
            "durableSequence": 1,
            "update": {
                "type": "committed",
                "event": {
                    "type": "threadCreated",
                    "sessionId": "session-1",
                    "threadId": "thread-1",
                    "title": "Thread"
                }
            }
        }),
    );
    let success = JsonRpcSuccess::new(JsonRpcId::Number(7), serde_json::json!({}));
    let failure = JsonRpcFailure::new(
        JsonRpcId::Null(()),
        serde_json::json!({"code": -32600, "message": "InvalidRequest", "data": null}),
    );

    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({"jsonrpc": "2.0", "id": 7, "method": "session/list", "params": {}}),
    );
    assert_eq!(
        serde_json::to_value(notification).unwrap(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "session/thread/update",
            "params": {
                "sessionId": "session-1",
                "threadId": "thread-1",
                "durableSequence": 1,
                "update": {
                    "type": "committed",
                    "event": {
                        "type": "threadCreated",
                        "sessionId": "session-1",
                        "threadId": "thread-1",
                        "title": "Thread"
                    }
                }
            }
        }),
    );
    assert_eq!(
        serde_json::to_value(success).unwrap(),
        serde_json::json!({"jsonrpc": "2.0", "id": 7, "result": {}}),
    );
    assert_eq!(
        serde_json::to_value(failure).unwrap(),
        serde_json::json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32600, "message": "InvalidRequest", "data": null}}),
    );
}

#[test]
fn dto_driven_typescript_preserves_model_ref_and_patch_shape() {
    let typescript = typescript();

    assert!(typescript.contains("export type ModelRef = { provider: string, model: string, };"));
    assert!(typescript.contains(
        "export type ModelCatalogEntry = { model: ModelRef, displayName: string, access: ModelAccess, outputTransport: ModelOutputTransport, contextWindow: number | null, autoCompactTokenLimit: number | null, availableContextWindow?: number | null, capabilities: ModelCapabilities, supportedReasoningEfforts: Array<ReasoningEffort>, defaultReasoningEffort: ReasoningEffort | null, defaultPersonality: Personality | null, };"
    ));
    assert!(
        typescript
            .contains("export type ApprovalReviewModelSelection = { \"type\": \"automatic\" }")
    );
    assert!(typescript.contains("preferredModel: ModelRef | null"));
    assert!(typescript.contains("preferredModel?: ModelRef | null"));
    assert!(typescript.contains("approvalReviewModel: ApprovalReviewModelSelection"));
    assert!(typescript.contains("approvalReviewModel?: ApprovalReviewModelSelection | null"));
    assert!(
        typescript.contains("export type ActionApprovalDecision = \"approveOnce\" | \"decline\"")
    );
    assert!(typescript.contains("{ \"type\": \"approval\", request: ActionApprovalRequest, }"));
    assert!(typescript.contains("{ \"type\": \"approval\", response: ActionApprovalResponse, }"));
    assert!(typescript.contains("expectedRevision: number"));
    assert!(typescript.contains("export type ProviderConfigDto ="));
    assert!(typescript.contains("export type ModelContextConfigDto ="));
    assert!(typescript.contains("modelContext?: { [key in string]: ModelContextConfigDto }"));
    assert!(typescript.contains(r#""provider/configure": { method: "provider/configure" }"#));
    assert!(typescript.contains(r#""provider/list": { method: "provider/list" }"#));
    assert!(typescript.contains(r#""provider/apiKey/set": { method: "provider/apiKey/set" }"#));
    assert!(typescript.contains("export type ProviderCatalogEntryDto ="));
    assert!(typescript.contains("export type McpServerConfigDto ="));
    assert!(typescript.contains("credentialRef: string"));
    assert!(typescript.contains("export type SkillSourceConfigDto ="));
    assert!(typescript.contains("export type PluginRequestDto ="));
    assert!(typescript.contains("export type HookConfigDto ="));
    assert!(typescript.contains(r#""mcp/server/upsert": { method: "mcp/server/upsert" }"#));
    assert!(typescript.contains(r#""skill/source/add": { method: "skill/source/add" }"#));
    assert!(typescript.contains(r#""plugin/request/upsert": { method: "plugin/request/upsert" }"#));
    assert!(typescript.contains(r#""hook/upsert": { method: "hook/upsert" }"#));
    assert!(typescript.contains("export type SkillName = string;"));
    assert!(typescript.contains("export type SkillSourceId = string;"));
    assert!(typescript.contains("export type ContentDigest = string;"));
    assert!(typescript.contains("export type SkillRef ="));
    assert!(typescript.contains("export type FrozenSkillActivation ="));
    assert!(typescript.contains("export type ContextCheckpoint ="));
    assert!(typescript.contains(r#"{ "type": "skill", skill: SkillRef, }"#));
    assert!(typescript.contains(r#""skills/list": { method: "skills/list" }"#));
    assert!(typescript.contains(r#""skill/resource/open": { method: "skill/resource/open" }"#));
    assert!(typescript.contains(r#""skills/changed": { method: "skills/changed" }"#));
    assert!(typescript.contains(r#""git/statusChanged": { method: "git/statusChanged" }"#));
    assert!(!typescript.contains("preferredModel: string"));
    assert!(typescript.contains(r#""type": "toolResult""#));
    assert!(typescript.contains("export type ToolName = string;"));
    assert!(typescript.contains("items: Array<ThreadItem>"));
    assert!(!typescript.contains("items?: Array<ThreadItem>"));
    assert!(typescript.contains("export type Session ="));
    assert!(typescript.contains("parentSequence: number"));
    assert!(typescript.contains("export type ToolExecutionAuthority ="));
    assert!(typescript.contains(r#"{ "type": "autoReviewed", assessmentId: string, }"#));
    assert!(typescript.contains("export type ProcessExecutionOutput ="));
    assert!(typescript.contains("export type SandboxDenialOutput ="));
    assert!(typescript.contains("replaySafety: ToolReplaySafety"));
    assert!(typescript.contains("dataBase64: string"));
    assert!(typescript.contains("decodedLength: number"));
    assert!(!typescript.contains(
        "ResourceReadResult = { resourceId: string, offset: number, data: Array<number>"
    ));
    assert!(typescript.contains("export const APP_SERVER_METHODS:"));
    assert!(typescript.contains(r#""session/create": { method: "session/create" }"#));
    assert!(typescript.contains(r#""session/request": { method: "session/request" }"#));
    assert!(typescript.contains(r#""session/thread/read": { method: "session/thread/read" }"#));
    assert!(
        typescript
            .contains(r#""session/thread/subscribe": { method: "session/thread/subscribe" }"#)
    );
    assert!(
        typescript
            .contains(r#""session/thread/unsubscribe": { method: "session/thread/unsubscribe" }"#)
    );
    assert!(!typescript.contains(r#""turn/start": { method: "turn/start" }"#));
    assert!(!typescript.contains(r#""turn/shell/start": { method: "turn/shell/start" }"#));
    assert!(typescript.contains(
        r#"export type InputItem = { "type": "text", text: string, } | { "type": "context", name: string, content: string, } | { "type": "imageAttachment", attachment: ImageAttachmentRef, } | { "type": "image", url: string, } | { "type": "skill", skill: SkillRef, };"#
    ));
    assert!(!typescript.contains("InputItemKind"));
    assert!(typescript.contains(r#"{ "type": "userImage""#));
    assert!(typescript.contains(r#"{ "type": "userImageAttachment""#));
    assert!(
        !typescript
            .contains(r#""turn/interaction/resolve": { method: "turn/interaction/resolve" }"#)
    );
    assert!(
        typescript.contains(r#""document/typst/compile": { method: "document/typst/compile" }"#)
    );
    assert!(typescript.contains(r#""fs/getMetadata": { method: "fs/getMetadata" }"#));
    assert!(typescript.contains(r#""fs/readDirectory": { method: "fs/readDirectory" }"#));
    assert!(typescript.contains(r#""fs/readFile": { method: "fs/readFile" }"#));
    assert!(typescript.contains(r#""fs/readBinaryFile": { method: "fs/readBinaryFile" }"#));
    assert!(typescript.contains(r#""fs/writeFile": { method: "fs/writeFile" }"#));
    assert!(typescript.contains("export type WorkspaceSessionDirectorySelector ="));
    assert!(typescript.contains("export type WorkspaceAdditionalDirectoryContributionsDto ="));
    assert!(typescript.contains(r#""fs/changed": { method: "fs/changed" }"#));
    assert!(
        typescript.contains(r#""workspace/search/start": { method: "workspace/search/start" }"#)
    );
    assert!(
        typescript
            .contains(r#""workspace/codeIndex/search": { method: "workspace/codeIndex/search" }"#)
    );
    assert!(
        typescript.contains(
            r#""workspace/codeIndex/retrieve": { method: "workspace/codeIndex/retrieve" }"#
        )
    );
    assert!(typescript.contains(
        r#""workspace/codeIndex/cloud/status": { method: "workspace/codeIndex/cloud/status" }"#
    ));
    assert!(typescript.contains(
        r#""workspace/codeIndex/cloud/preview": { method: "workspace/codeIndex/cloud/preview" }"#
    ));
    assert!(typescript.contains(
        r#""workspace/codeIndex/cloud/authorize": { method: "workspace/codeIndex/cloud/authorize" }"#
    ));
    assert!(typescript.contains(
        r#""workspace/codeIndex/cloud/sync": { method: "workspace/codeIndex/cloud/sync" }"#
    ));
    assert!(typescript.contains(
        r#""workspace/codeIndex/cloud/revoke": { method: "workspace/codeIndex/cloud/revoke" }"#
    ));
    assert!(typescript.contains(r#""terminal/profile/list": { method: "terminal/profile/list" }"#));
    assert!(typescript.contains(r#""terminal/create": { method: "terminal/create" }"#));
    assert!(typescript.contains(r#""terminal/attach": { method: "terminal/attach" }"#));
    assert!(typescript.contains(r#""terminal/read": { method: "terminal/read" }"#));
    assert!(typescript.contains("export type TerminalProfile ="));
    assert!(typescript.contains("profileId: string"));
    assert!(typescript.contains("export type TerminalProfileSelection ="));
    assert!(typescript.contains("export type TerminalProfileListResult ="));
    assert!(typescript.contains("export type TerminalLifecycle ="));
    assert!(typescript.contains("export type TerminalReconnectLease ="));
    assert!(typescript.contains("export type TerminalAttachParams ="));
    assert!(typescript.contains("export type TerminalAttachResult ="));
    assert!(typescript.contains("export type TerminalCommandStatus ="));
    assert!(typescript.contains("export type TerminalCommandStatusEvent ="));
    assert!(typescript.contains("export type TerminalReadResult ="));
    assert!(typescript.contains("export type WorkspaceSearchMatch ="));
    assert!(typescript.contains("export type CodeIndexStatusResult ="));
    assert!(typescript.contains("export type CodeIndexSearchResult ="));
    assert!(typescript.contains("export type CodeRetrievalResult ="));
    assert!(typescript.contains("rrfScore: number"));
    assert!(
        typescript.contains("export type CodeIndexDeploymentModeDto = \"localOnly\" | \"cloud\";")
    );
    assert!(!typescript.contains("CloudCodeIndexModeDto"));
    assert!(!typescript.contains("cloudManaged"));
    assert!(typescript.contains("export type CloudCodeIndexSelectionDto ="));
    assert!(typescript.contains("export type CloudCodeIndexStatusResult ="));
    assert!(typescript.contains("syncedLocalGeneration: number | null"));
    assert!(typescript.contains("export type TypstCompileResult ="));
    assert!(typescript.contains(r#""status": "success""#));
    assert!(typescript.contains("export type TurnInteraction ="));
    assert!(typescript.contains("export type PendingInteraction ="));
    assert!(
        typescript.contains("export type ToolMode = \"direct\" | \"codeMode\" | \"codeModeOnly\";")
    );
    assert!(typescript.contains("export const APP_SERVER_NOTIFICATIONS:"));
    assert!(!typescript.contains("ThreadStartParams"));
}

#[test]
fn dto_driven_schema_contains_registered_rpc_envelopes() {
    let schema: serde_json::Value = serde_json::from_str(&json_schema()).unwrap();
    let definitions = schema["$defs"]
        .as_object()
        .expect("generated schema should contain shared definitions");

    assert!(definitions.contains_key("JsonRpcRequest"));
    assert!(definitions.contains_key("JsonRpcResponse"));
    assert!(definitions.contains_key("JsonRpcNotification"));
    assert!(definitions.contains_key("ModelRefDto"));
    assert!(definitions.contains_key("ApprovalReviewModelSelectionDto"));
    assert!(definitions.contains_key("ActionApprovalRequest"));
    assert!(definitions.contains_key("ActionApprovalResponse"));
    assert!(definitions.contains_key("McpServerConfigDto"));
    assert!(definitions.contains_key("SkillSourceConfigDto"));
    assert!(definitions.contains_key("Session"));
    assert!(definitions.contains_key("ThreadItem"));
    assert!(definitions.contains_key("ToolMode"));
    assert!(definitions.contains_key("TypstCompileParams"));
    assert!(definitions.contains_key("TypstCompileResult"));
    assert!(definitions.contains_key("WorkspaceSearchStartParams"));
    assert!(definitions.contains_key("WorkspaceSearchReadResult"));
    assert!(definitions.contains_key("CodeIndexStatusResult"));
    assert!(definitions.contains_key("CodeIndexSearchParams"));
    assert!(definitions.contains_key("CodeIndexSearchResult"));
    assert!(definitions.contains_key("CodeRetrievalParams"));
    assert!(definitions.contains_key("CodeRetrievalResult"));
    assert!(definitions.contains_key("TerminalProfile"));
    assert!(definitions.contains_key("TerminalProfileSelection"));
    assert!(definitions.contains_key("TerminalProfileListResult"));
    assert!(definitions.contains_key("TerminalCreateParams"));
    assert!(definitions.contains_key("TerminalLifecycle"));
    assert!(definitions.contains_key("TerminalReconnectLease"));
    assert!(definitions.contains_key("TerminalAttachParams"));
    assert!(definitions.contains_key("TerminalAttachResult"));
    assert!(definitions.contains_key("TerminalCommandStatus"));
    assert!(definitions.contains_key("TerminalCommandStatusEvent"));
    assert!(definitions.contains_key("TerminalReadResult"));
    assert!(definitions.contains_key("GitStatusResult"));
    assert!(definitions.contains_key("GitPathsParams"));
    assert!(definitions.contains_key("GitCommitParams"));
    assert!(definitions.contains_key("GitOperationResult"));
    assert!(definitions.contains_key("GitCommitResult"));
    assert_eq!(definitions["ThreadId"]["minLength"], 1);
    assert_eq!(definitions["SessionId"]["minLength"], 1);
    assert_eq!(definitions["CommandId"]["minLength"], 1);
    let start_turn_request = definitions["SessionRequest"]["oneOf"]
        .as_array()
        .expect("SessionRequest should be a tagged union")
        .iter()
        .find(|request| request["properties"]["type"]["const"] == "startTurn")
        .expect("SessionRequest should contain startTurn");
    assert_eq!(start_turn_request["properties"]["input"]["minItems"], 1);
    assert_eq!(
        definitions["ResourceReadParams"]["properties"]["maxBytes"]["maximum"],
        262_144
    );
    assert!(definitions["ResourceReadResult"]["properties"]["dataBase64"].is_object());
    assert_eq!(
        definitions["ResourceReadResult"]["properties"]["decodedLength"]["maximum"],
        262_144
    );
    assert!(
        definitions["ResourceReadResult"]["properties"]
            .get("data")
            .is_none()
    );
}

#[test]
fn config_patch_fixture_round_trips_the_provider_scoped_model() {
    let fixture = serde_json::json!({
        "commandId": "config-model",
        "expectedRevision": 4,
        "preferredModel": {
            "provider": "openai",
            "model": "gpt-5.6"
        },
        "approvalReviewModel": {
            "type": "explicit",
            "model": {
                "provider": "openai",
                "model": "codex-auto-review"
            }
        }
    });
    let params: ConfigUpdateParams = serde_json::from_value(fixture.clone()).unwrap();

    assert!(matches!(
        &params.preferred_model,
        Patch::Value(model) if model.provider == "openai"
    ));
    assert!(matches!(
        &params.approval_review_model,
        Patch::Value(ApprovalReviewModelSelectionDto::Explicit { model })
            if model.model == "codex-auto-review"
    ));
    assert_eq!(params.expected_revision, 4);
    assert_eq!(serde_json::to_value(params).unwrap(), fixture);
}

#[test]
fn config_patch_distinguishes_missing_null_and_value() {
    let missing: ConfigUpdateParams = serde_json::from_value(serde_json::json!({
        "commandId": "missing",
        "expectedRevision": 0
    }))
    .unwrap();
    let null: ConfigUpdateParams = serde_json::from_value(serde_json::json!({
        "commandId": "null",
        "expectedRevision": 3,
        "preferredModel": null,
        "approvalReviewModel": null
    }))
    .unwrap();

    assert_eq!(missing.preferred_model, Patch::Missing);
    assert_eq!(missing.approval_review_model, Patch::Missing);
    assert_eq!(missing.expected_revision, 0);
    assert_eq!(null.preferred_model, Patch::Null);
    assert_eq!(null.approval_review_model, Patch::Null);
    assert_eq!(
        serde_json::to_value(missing).unwrap(),
        serde_json::json!({"commandId": "missing", "expectedRevision": 0})
    );
    assert_eq!(
        serde_json::to_value(null).unwrap(),
        serde_json::json!({
            "commandId": "null",
            "expectedRevision": 3,
            "preferredModel": null,
            "approvalReviewModel": null
        })
    );
}

#[test]
fn mcp_and_skill_config_commands_round_trip() {
    let mcp_fixture = serde_json::json!({
        "commandId": "github-mcp",
        "expectedRevision": 7,
        "server": {
            "id": "user:mcp:github",
            "displayName": "GitHub",
            "transport": {"type": "streamableHttp", "url": "https://mcp.github.example"},
            "credential": {"type": "reference", "credentialRef": "user:credential:github"},
            "enablement": "disabled"
        }
    });
    let skill_fixture = serde_json::json!({
        "commandId": "personal-skills",
        "expectedRevision": 8,
        "source": {
            "id": "user:skill-source:personal",
            "rootReference": "user:skill-root:personal",
            "enablement": "enabled"
        }
    });

    let mcp: McpServerUpsertParams = serde_json::from_value(mcp_fixture.clone()).unwrap();
    let skill: SkillSourceAddParams = serde_json::from_value(skill_fixture.clone()).unwrap();

    assert_eq!(serde_json::to_value(mcp).unwrap(), mcp_fixture);
    assert_eq!(serde_json::to_value(skill).unwrap(), skill_fixture);
}

#[test]
fn exec_policy_rule_command_round_trips_recursive_typed_selectors() {
    let fixture = serde_json::json!({
        "commandId": "allow-git-status",
        "expectedRevision": 9,
        "rule": {
            "id": "allow-git-status",
            "selector": {
                "type": "all",
                "selectors": [
                    {
                        "type": "source",
                        "source": "built_in_tool",
                        "sourceId": "shell-command"
                    },
                    {
                        "type": "commandPrefix",
                        "pattern": [
                            {"type": "literal", "value": "git"},
                            {"type": "oneOf", "value": ["status", "diff"]}
                        ]
                    }
                ]
            },
            "effect": {"type": "allowUnsandboxed"},
            "justification": "explicit user rule"
        }
    });
    let params: ExecPolicyRuleUpsertParams = serde_json::from_value(fixture.clone()).unwrap();

    assert_eq!(serde_json::to_value(params).unwrap(), fixture);
}

#[test]
fn durable_events_without_model_snapshots_remain_readable() {
    let session: SessionEvent = serde_json::from_value(serde_json::json!({
        "type": "sessionCreated",
        "sessionId": "session-1",
        "title": "Legacy session"
    }))
    .unwrap();
    let turn: ThreadEvent = serde_json::from_value(serde_json::json!({
        "type": "turnAccepted",
        "threadId": "thread-1",
        "turnId": "turn-1"
    }))
    .unwrap();

    assert!(matches!(
        session,
        SessionEvent::SessionCreated { model: None, .. }
    ));
    assert!(matches!(
        turn,
        ThreadEvent::TurnAccepted { model: None, .. }
    ));
}

#[test]
fn schema_hash_is_stable_sha256_of_the_generated_schema() {
    let first = schema_hash();
    let second = schema_hash();

    assert_eq!(first, second);
    assert_eq!(first.len(), "sha256:".len() + 64);
    assert!(first.starts_with("sha256:"));
}

#[test]
fn schema_fixtures_match_the_generators() {
    let typescript_fixture = include_str!("../schema/typescript/types.ts");
    let schema = include_str!("../schema/json/schema.json");

    assert_eq!(typescript_fixture.replace("\r\n", "\n"), typescript());
    assert_eq!(schema.replace("\r\n", "\n"), json_schema());
}
