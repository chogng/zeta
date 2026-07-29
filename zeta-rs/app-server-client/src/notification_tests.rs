use super::{ServerNotification, decode};
use zeta_app_server_protocol::protocol::git::{GitChangeStatusDto, GitHeadDto};

#[test]
fn decodes_git_status_changed_notification() {
    let notification = decode(
        r#"{
            "jsonrpc": "2.0",
            "method": "git/statusChanged",
            "params": {
                "status": {
                    "streamInstanceId": "git_stream_1",
                    "revision": 7,
                    "head": {
                        "type": "branch",
                        "name": "main",
                        "objectId": "0123456789abcdef",
                        "upstream": null
                    },
                    "changes": [{
                        "path": "src/lib.rs",
                        "originalPath": null,
                        "indexStatus": "unmodified",
                        "worktreeStatus": "modified",
                        "conflicted": false,
                        "submodule": {
                            "isSubmodule": false,
                            "commitChanged": false,
                            "trackedChanges": false,
                            "untrackedChanges": false
                        }
                    }]
                }
            }
        }"#,
    )
    .expect("git status notification decodes");

    let ServerNotification::GitStatusChanged(changed) = notification else {
        panic!("expected git status notification");
    };
    assert_eq!(changed.status.stream_instance_id.as_str(), "git_stream_1");
    assert_eq!(changed.status.revision, 7);
    assert!(matches!(
        changed.status.head,
        GitHeadDto::Branch { ref name, .. } if name == "main"
    ));
    assert_eq!(
        changed.status.changes[0].worktree_status,
        GitChangeStatusDto::Modified
    );
}
